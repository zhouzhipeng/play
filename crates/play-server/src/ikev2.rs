use std::ffi::OsStr;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, DistinguishedName, DnType,
    ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair, KeyUsagePurpose, RsaKeySize, SanType,
    PKCS_RSA_SHA256,
};
use tokio::fs;
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

use crate::config::Ikev2ServerConfig;
use play_shared::constants::DATA_DIR;

#[cfg(all(feature = "ikev2-server", target_os = "linux"))]
use std::process::Stdio;
#[cfg(all(feature = "ikev2-server", target_os = "linux"))]
use tokio::io::{AsyncBufReadExt, BufReader};
#[cfg(all(feature = "ikev2-server", target_os = "linux"))]
use tokio::process::{Child, Command};
#[cfg(all(feature = "ikev2-server", target_os = "linux"))]
use tokio::time::{sleep, Duration, Instant};

pub struct Ikev2ServerHandle {
    #[cfg(all(feature = "ikev2-server", target_os = "linux"))]
    child: Child,
    #[cfg(all(feature = "ikev2-server", target_os = "linux"))]
    _runtime_dir: tempfile::TempDir,
}

pub struct Ikev2BackgroundHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: tokio::task::JoinHandle<()>,
}

impl Ikev2BackgroundHandle {
    pub async fn shutdown(mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Err(error) = self.join_handle.await {
            warn!("IKEv2 background task join failed: {}", error);
        }
    }
}

impl Ikev2ServerHandle {
    pub async fn shutdown(mut self) {
        #[cfg(all(feature = "ikev2-server", target_os = "linux"))]
        {
            if let Err(error) = self.child.start_kill() {
                warn!("Failed to stop IKEv2 daemon child: {}", error);
            }

            if let Err(error) = self.child.wait().await {
                warn!("Failed to wait for IKEv2 daemon child exit: {}", error);
            }
        }
    }
}

pub fn maybe_start_ikev2_server_in_background(
    config: &Ikev2ServerConfig,
) -> Option<Ikev2BackgroundHandle> {
    if !config.enabled {
        return None;
    }

    let config = config.clone();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    let join_handle = tokio::spawn(async move {
        info!("Starting embedded IKEv2 service in background");

        let startup = maybe_start_ikev2_server(&config);
        tokio::pin!(startup);

        let handle = tokio::select! {
            result = &mut startup => {
                match result {
                    Ok(handle) => handle,
                    Err(error) => {
                        error_background_startup(&error);
                        return;
                    }
                }
            }
            _ = &mut shutdown_rx => {
                info!("Embedded IKEv2 background startup cancelled before completion");
                return;
            }
        };

        let Some(handle) = handle else {
            return;
        };

        let _ = (&mut shutdown_rx).await;
        handle.shutdown().await;
    });

    Some(Ikev2BackgroundHandle {
        shutdown_tx: Some(shutdown_tx),
        join_handle,
    })
}

pub async fn maybe_start_ikev2_server(
    config: &Ikev2ServerConfig,
) -> Result<Option<Ikev2ServerHandle>> {
    if !config.enabled {
        return Ok(None);
    }

    #[cfg(all(feature = "ikev2-server", target_os = "linux"))]
    {
        let binaries = ensure_runtime_binaries(config).await?;
        if config.auto_install_dependencies {
            stop_conflicting_strongswan_services().await;
        }
        ensure_credentials_exist(config).await?;
        validate_config(config).await?;

        let runtime = write_runtime_files(config).await?;
        let vici_uri = format!("unix://{}", runtime.vici_socket_path.display());

        let mut child = Command::new(&binaries.daemon_bin);
        child
            .env("STRONGSWAN_CONF", &runtime.strongswan_conf_path)
            .current_dir(runtime.root.path())
            .kill_on_drop(true)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = child
            .spawn()
            .with_context(|| format!("start IKEv2 daemon binary `{}`", binaries.daemon_bin.display()))?;

        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    log_ikev2_daemon_line("stdout", &line);
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    log_ikev2_daemon_line("stderr", &line);
                }
            });
        }

        wait_until_ready(config, &runtime, &vici_uri, &binaries, &mut child).await?;

        info!(
            "Embedded IKEv2 service started with connection `{}` on {}:{} / {}:{}",
            config.connection_name,
            config.listen_addr,
            config.port,
            config.listen_addr,
            config.port_nat_t
        );

        return Ok(Some(Ikev2ServerHandle {
            child,
            _runtime_dir: runtime.root,
        }));
    }

    #[cfg(all(not(feature = "ikev2-server"), target_os = "linux"))]
    {
        warn!(
            "ikev2_server.enabled is true, but play-server was built without the `ikev2-server` feature. The IKEv2 config is ignored"
        );
        Ok(None)
    }

    #[cfg(not(target_os = "linux"))]
    {
        warn!(
            "ikev2_server.enabled is true, but embedded IKEv2 runtime support is currently Linux-only. The config is ignored on this platform"
        );
        Ok(None)
    }
}

fn error_background_startup(error: &anyhow::Error) {
    warn!(
        "Embedded IKEv2 startup failed in background; HTTP server continues running without IKEv2: {error:#}"
    );
}

fn log_ikev2_daemon_line(stream: &str, line: &str) {
    if looks_like_ikev2_problem(line) {
        warn!("ikev2-daemon {}: {}", stream, line);
    } else {
        debug!("ikev2-daemon {}: {}", stream, line);
    }
}

fn looks_like_ikev2_problem(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    [
        "error",
        "fail",
        "fatal",
        "unable",
        "denied",
        "timed out",
        "timeout",
        "no such file",
        "not found",
        "invalid",
        "refused",
    ]
    .iter()
    .any(|needle| line.contains(needle))
}

#[cfg(all(feature = "ikev2-server", target_os = "linux"))]
async fn wait_until_ready(
    config: &Ikev2ServerConfig,
    runtime: &RuntimeFiles,
    vici_uri: &str,
    binaries: &ResolvedRuntimeBinaries,
    child: &mut Child,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(config.startup_timeout_secs.max(1));
    let mut last_error = None;

    loop {
        if let Some(status) = child.try_wait().context("poll IKEv2 daemon child failed")? {
            bail!("IKEv2 daemon exited before it became ready: {}", status);
        }

        if runtime.vici_socket_path.exists() {
            match load_runtime_config(config, runtime, vici_uri, binaries).await {
                Ok(()) => return Ok(()),
                Err(error) => last_error = Some(error),
            }
        }

        if Instant::now() >= deadline {
            let detail = last_error
                .map(|error| format!("{error:#}"))
                .unwrap_or_else(|| {
                    format!(
                        "VICI socket {} did not become ready in time",
                        runtime.vici_socket_path.display()
                    )
                });
            bail!("embedded IKEv2 startup timed out: {}", detail);
        }

        sleep(Duration::from_millis(250)).await;
    }
}

#[cfg(all(feature = "ikev2-server", target_os = "linux"))]
async fn load_runtime_config(
    config: &Ikev2ServerConfig,
    runtime: &RuntimeFiles,
    vici_uri: &str,
    binaries: &ResolvedRuntimeBinaries,
) -> Result<()> {
    let output = Command::new(&binaries.swanctl_bin)
        .arg("--load-all")
        .arg("--file")
        .arg(&runtime.swanctl_conf_path)
        .arg("--uri")
        .arg(vici_uri)
        .env("SWANCTL_DIR", &runtime.swanctl_dir)
        .output()
        .await
        .with_context(|| {
            format!(
                "run `{}` to load IKEv2 config",
                binaries.swanctl_bin.display()
            )
        })?;

    if output.status.success() {
        return Ok(());
    }

    bail!(
        "{} --load-all failed: stdout=`{}` stderr=`{}`",
        binaries.swanctl_bin.display(),
        String::from_utf8_lossy(&output.stdout).trim(),
        String::from_utf8_lossy(&output.stderr).trim()
    );
}

#[derive(Debug)]
struct RuntimeFiles {
    root: tempfile::TempDir,
    swanctl_dir: PathBuf,
    strongswan_conf_path: PathBuf,
    swanctl_conf_path: PathBuf,
    vici_socket_path: PathBuf,
}

#[derive(Debug)]
struct ResolvedRuntimeBinaries {
    daemon_bin: PathBuf,
    swanctl_bin: PathBuf,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct OsReleaseInfo {
    id: String,
    version_id: Option<String>,
    version_codename: Option<String>,
}

fn resolve_runtime_binaries(config: &Ikev2ServerConfig) -> Result<ResolvedRuntimeBinaries> {
    let daemon_bin = if config.daemon_bin == "charon-systemd" {
        resolve_executable_with_fallbacks(&config.daemon_bin, &["charon"])?
    } else {
        resolve_executable(&config.daemon_bin)?
    };
    let swanctl_bin = resolve_executable(&config.swanctl_bin)?;

    if daemon_bin.file_name() != Some(OsStr::new(&config.daemon_bin)) {
        warn!(
            "Configured IKEv2 daemon binary `{}` not found, using fallback `{}`",
            config.daemon_bin,
            daemon_bin.display()
        );
    }

    Ok(ResolvedRuntimeBinaries {
        daemon_bin,
        swanctl_bin,
    })
}

#[cfg(all(feature = "ikev2-server", target_os = "linux"))]
async fn ensure_runtime_binaries(config: &Ikev2ServerConfig) -> Result<ResolvedRuntimeBinaries> {
    match resolve_runtime_binaries(config) {
        Ok(binaries) => Ok(binaries),
        Err(initial_error) => {
            if !config.auto_install_dependencies {
                return Err(initial_error.context(
                    "IKEv2 runtime binaries are missing and automatic installation is disabled",
                ));
            }

            info!(
                "IKEv2 runtime binaries are missing; attempting automatic install on Debian Bookworm: {initial_error:#}"
            );

            ensure_supported_auto_install_host().await?;
            install_ikev2_runtime_dependencies().await?;

            resolve_runtime_binaries(config).with_context(|| {
                format!(
                    "IKEv2 runtime binaries are still unavailable after automatic installation. Initial detection failed with: {initial_error:#}"
                )
            })
        }
    }
}

#[cfg(all(feature = "ikev2-server", target_os = "linux"))]
async fn ensure_supported_auto_install_host() -> Result<()> {
    let os_release = fs::read_to_string("/etc/os-release")
        .await
        .context("read /etc/os-release for IKEv2 dependency auto-install")?;
    let info = parse_os_release(&os_release);

    let is_supported = info.id == "debian"
        && (info.version_codename.as_deref() == Some("bookworm")
            || info.version_id.as_deref() == Some("12"));

    if is_supported {
        return Ok(());
    }

    bail!(
        "IKEv2 dependency auto-install only supports Debian Bookworm. Detected ID=`{}` VERSION_ID=`{}` VERSION_CODENAME=`{}`",
        info.id,
        info.version_id.as_deref().unwrap_or(""),
        info.version_codename.as_deref().unwrap_or("")
    );
}

#[cfg(all(feature = "ikev2-server", target_os = "linux"))]
async fn install_ikev2_runtime_dependencies() -> Result<()> {
    let apt_get = resolve_apt_get_binary()?;
    info!(
        "Installing IKEv2 runtime dependencies with `{}`",
        apt_get.display()
    );

    run_command_checked(
        Command::new(&apt_get)
            .env("DEBIAN_FRONTEND", "noninteractive")
            .env("APT_LISTCHANGES_FRONTEND", "none")
            .env("NEEDRESTART_MODE", "a")
            .arg("update"),
        "update Debian package indexes for IKEv2 bootstrap",
    )
    .await?;

    run_command_checked(
        Command::new(&apt_get)
            .env("DEBIAN_FRONTEND", "noninteractive")
            .env("APT_LISTCHANGES_FRONTEND", "none")
            .env("NEEDRESTART_MODE", "a")
            .arg("install")
            .arg("-y")
            .arg("--no-install-recommends")
            .arg("charon-systemd")
            .arg("strongswan-swanctl")
            .arg("libcharon-extauth-plugins"),
        "install IKEv2 runtime dependencies",
    )
    .await
}

#[cfg(all(feature = "ikev2-server", target_os = "linux"))]
async fn run_command_checked(command: &mut Command, description: &str) -> Result<()> {
    let output = command
        .output()
        .await
        .with_context(|| format!("failed to {description}"))?;

    if output.status.success() {
        return Ok(());
    }

    bail!(
        "{}: stdout=`{}` stderr=`{}`",
        description,
        String::from_utf8_lossy(&output.stdout).trim(),
        String::from_utf8_lossy(&output.stderr).trim()
    );
}

#[cfg(all(feature = "ikev2-server", target_os = "linux"))]
fn resolve_apt_get_binary() -> Result<PathBuf> {
    resolve_executable("/usr/bin/apt-get").or_else(|_| resolve_executable("apt-get"))
}

#[cfg(all(feature = "ikev2-server", target_os = "linux"))]
async fn stop_conflicting_strongswan_services() {
    let systemctl = match resolve_executable("/usr/bin/systemctl")
        .or_else(|_| resolve_executable("/bin/systemctl"))
        .or_else(|_| resolve_executable("systemctl"))
    {
        Ok(path) => path,
        Err(_) => return,
    };

    for unit in [
        "strongswan.service",
        "strongswan-starter.service",
        "strongswan-swanctl.service",
        "charon-systemd.service",
    ] {
        match Command::new(&systemctl)
            .arg("disable")
            .arg("--now")
            .arg("--quiet")
            .arg(unit)
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                info!("Disabled conflicting strongSwan service `{}`", unit);
            }
            Ok(_) => {}
            Err(error) => warn!(
                "Failed to invoke `{}` for `{}`: {}",
                systemctl.display(),
                unit,
                error
            ),
        }
    }
}

fn resolve_executable_with_fallbacks(primary: &str, fallbacks: &[&str]) -> Result<PathBuf> {
    resolve_executable_with_fallbacks_in_path(primary, fallbacks, std::env::var_os("PATH").as_deref())
}

fn resolve_executable_with_fallbacks_in_path(
    primary: &str,
    fallbacks: &[&str],
    path_var: Option<&OsStr>,
) -> Result<PathBuf> {
    let mut candidates = Vec::with_capacity(fallbacks.len() + 1);
    candidates.push(primary);
    candidates.extend(fallbacks.iter().copied());

    for candidate in candidates {
        if let Some(path) = lookup_executable_in_path(candidate, path_var) {
            return Ok(path);
        }
    }

    bail!(
        "unable to find IKEv2 executable `{}` in PATH. Tried fallbacks: {}",
        primary,
        fallbacks.join(", ")
    )
}

fn resolve_executable(command: &str) -> Result<PathBuf> {
    lookup_executable(command)
        .ok_or_else(|| anyhow!("unable to find IKEv2 executable `{command}` in PATH"))
}

fn lookup_executable(command: &str) -> Option<PathBuf> {
    let candidate = PathBuf::from(command);
    if candidate.components().count() > 1 {
        return candidate.is_file().then_some(candidate);
    }

    lookup_executable_in_path(command, std::env::var_os("PATH").as_deref())
}

fn lookup_executable_in_path(command: &str, path_var: Option<&OsStr>) -> Option<PathBuf> {
    let path_var = path_var?;
    for directory in std::env::split_paths(&path_var) {
        let full_path = directory.join(command);
        if full_path.is_file() {
            return Some(full_path);
        }
    }

    None
}

fn parse_os_release(content: &str) -> OsReleaseInfo {
    let mut info = OsReleaseInfo::default();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = parse_os_release_value(value.trim());

        match key {
            "ID" => info.id = value,
            "VERSION_ID" => info.version_id = Some(value),
            "VERSION_CODENAME" => info.version_codename = Some(value),
            _ => {}
        }
    }

    info
}

fn parse_os_release_value(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|value| value.strip_suffix('\'')))
        .unwrap_or(value)
        .to_string()
}

async fn validate_config(config: &Ikev2ServerConfig) -> Result<()> {
    if config.local_id.trim().is_empty() {
        bail!("ikev2_server.local_id must not be empty");
    }
    if config.server_cert.trim().is_empty() {
        bail!("ikev2_server.server_cert must not be empty");
    }
    if config.server_key.trim().is_empty() {
        bail!("ikev2_server.server_key must not be empty");
    }
    if config.pool.trim().is_empty() {
        bail!("ikev2_server.pool must not be empty");
    }
    if config.eap_users.is_empty() {
        bail!("ikev2_server.eap_users must contain at least one user");
    }

    ensure_file_exists(&resolve_config_path(&config.server_cert)?).await?;
    ensure_file_exists(&resolve_config_path(&config.server_key)?).await?;

    if let Some(ca_cert) = &config.ca_cert {
        if !ca_cert.trim().is_empty() {
            ensure_file_exists(&resolve_config_path(ca_cert)?).await?;
        }
    }

    for (user, password) in &config.eap_users {
        if user.trim().is_empty() {
            bail!("ikev2_server.eap_users contains an empty user id");
        }
        if password.trim().is_empty() {
            bail!("ikev2_server.eap_users.{user} must not be empty");
        }
    }

    Ok(())
}

async fn ensure_credentials_exist(config: &Ikev2ServerConfig) -> Result<()> {
    let paths = resolved_credential_paths(config)?;
    ensure_parent_dirs(&paths).await?;

    let server_cert_exists = path_exists(&paths.server_cert).await;
    let server_key_exists = path_exists(&paths.server_key).await;
    let ca_cert_exists = path_exists(&paths.ca_cert).await;
    let ca_key_exists = path_exists(&paths.ca_key).await;

    if server_cert_exists && server_key_exists && ca_cert_exists {
        return Ok(());
    }

    if !server_cert_exists && !server_key_exists && !ca_cert_exists && !ca_key_exists {
        generate_full_certificate_bundle(config, &paths).await?;
        info!(
            "Auto-generated IKEv2 CA/server certificate bundle under {}",
            paths.bundle_dir.display()
        );
        return Ok(());
    }

    if ca_cert_exists && ca_key_exists && (!server_cert_exists || !server_key_exists) {
        generate_server_credentials_from_existing_ca(config, &paths).await?;
        info!(
            "Auto-generated IKEv2 server certificate from existing CA {}",
            paths.ca_cert.display()
        );
        return Ok(());
    }

    if server_cert_exists ^ server_key_exists {
        bail!(
            "IKEv2 server credentials are incomplete. Remove {} and {} to let play-server regenerate them automatically, or provide a matching custom pair",
            paths.server_cert.display(),
            paths.server_key.display()
        );
    }

    bail!(
        "IKEv2 certificate state is incomplete. Expected server cert {}, server key {}, CA cert {}, and optional CA key {}. Remove the partial files to let play-server auto-generate a fresh bundle, or provide a complete custom set",
        paths.server_cert.display(),
        paths.server_key.display(),
        paths.ca_cert.display(),
        paths.ca_key.display()
    );
}

async fn ensure_file_exists(path: &Path) -> Result<()> {
    fs::metadata(path)
        .await
        .with_context(|| format!("required IKEv2 file not found: {}", path.display()))?;
    Ok(())
}

async fn write_runtime_files(config: &Ikev2ServerConfig) -> Result<RuntimeFiles> {
    let root = tempfile::tempdir().context("create temporary IKEv2 runtime dir failed")?;
    let root_path = root.path().to_path_buf();
    let swanctl_dir = root_path.join("swanctl");
    let x509_dir = swanctl_dir.join("x509");
    let private_dir = swanctl_dir.join("private");
    let x509ca_dir = swanctl_dir.join("x509ca");
    let strongswan_conf_path = root_path.join("strongswan.conf");
    let swanctl_conf_path = swanctl_dir.join("swanctl.conf");
    let vici_socket_path = root_path.join("charon.vici");

    fs::create_dir_all(&x509_dir)
        .await
        .with_context(|| format!("create {}", x509_dir.display()))?;
    fs::create_dir_all(&private_dir)
        .await
        .with_context(|| format!("create {}", private_dir.display()))?;
    fs::create_dir_all(&x509ca_dir)
        .await
        .with_context(|| format!("create {}", x509ca_dir.display()))?;

    let server_cert_name = "play-server-cert.pem".to_string();
    let server_key_name = "play-server-key.pem".to_string();

    copy_file_into_dir(
        &resolve_config_path(&config.server_cert)?,
        &x509_dir,
        &server_cert_name,
    )
    .await?;
    copy_file_into_dir(
        &resolve_config_path(&config.server_key)?,
        &private_dir,
        &server_key_name,
    )
    .await?;

    if let Some(ca_cert) = &config.ca_cert {
        if !ca_cert.trim().is_empty() {
            copy_file_into_dir(
                &resolve_config_path(ca_cert)?,
                &x509ca_dir,
                "play-server-ca.pem",
            )
            .await?;
        }
    }

    let strongswan_conf = render_strongswan_conf(config, &vici_socket_path);
    fs::write(&strongswan_conf_path, strongswan_conf)
        .await
        .with_context(|| format!("write {}", strongswan_conf_path.display()))?;

    let swanctl_conf = render_swanctl_conf(config, &server_cert_name, &server_key_name);
    fs::write(&swanctl_conf_path, swanctl_conf)
        .await
        .with_context(|| format!("write {}", swanctl_conf_path.display()))?;

    Ok(RuntimeFiles {
        root,
        swanctl_dir,
        strongswan_conf_path,
        swanctl_conf_path,
        vici_socket_path,
    })
}

async fn copy_file_into_dir(source: &Path, target_dir: &Path, target_name: &str) -> Result<()> {
    let destination = target_dir.join(target_name);
    fs::copy(source, &destination)
        .await
        .with_context(|| format!("copy {} to {}", source.display(), destination.display()))?;
    Ok(())
}

#[derive(Debug)]
struct ResolvedCredentialPaths {
    bundle_dir: PathBuf,
    server_cert: PathBuf,
    server_key: PathBuf,
    ca_cert: PathBuf,
    ca_key: PathBuf,
}

fn resolved_credential_paths(config: &Ikev2ServerConfig) -> Result<ResolvedCredentialPaths> {
    let server_cert = resolve_config_path(&config.server_cert)?;
    let server_key = resolve_config_path(&config.server_key)?;
    let ca_cert = resolve_config_path(
        config
            .ca_cert
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("certs/ikev2/ca-cert.pem"),
    )?;
    let ca_key = derive_ca_key_path(&ca_cert);

    let bundle_dir = server_cert
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    Ok(ResolvedCredentialPaths {
        bundle_dir,
        server_cert,
        server_key,
        ca_cert,
        ca_key,
    })
}

async fn ensure_parent_dirs(paths: &ResolvedCredentialPaths) -> Result<()> {
    for path in [&paths.server_cert, &paths.server_key, &paths.ca_cert, &paths.ca_key] {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .with_context(|| format!("create {}", parent.display()))?;
        }
    }
    Ok(())
}

async fn path_exists(path: &Path) -> bool {
    fs::metadata(path).await.is_ok()
}

async fn generate_full_certificate_bundle(
    config: &Ikev2ServerConfig,
    paths: &ResolvedCredentialPaths,
) -> Result<()> {
    let (ca_cert_pem, ca_key_pem, issuer) = build_ca_issuer(config)?;
    write_string(&paths.ca_cert, &ca_cert_pem).await?;
    write_string(&paths.ca_key, &ca_key_pem).await?;
    generate_server_credentials(config, paths, &issuer).await?;
    Ok(())
}

async fn generate_server_credentials_from_existing_ca(
    config: &Ikev2ServerConfig,
    paths: &ResolvedCredentialPaths,
) -> Result<()> {
    let ca_cert_pem = fs::read_to_string(&paths.ca_cert)
        .await
        .with_context(|| format!("read {}", paths.ca_cert.display()))?;
    let ca_key_pem = fs::read_to_string(&paths.ca_key)
        .await
        .with_context(|| format!("read {}", paths.ca_key.display()))?;
    let ca_key_pair =
        KeyPair::from_pem(&ca_key_pem).context("parse generated IKEv2 CA private key failed")?;
    let issuer = Issuer::from_ca_cert_pem(&ca_cert_pem, ca_key_pair)
        .context("parse generated IKEv2 CA certificate failed")?;

    generate_server_credentials(config, paths, &issuer).await?;
    Ok(())
}

async fn generate_server_credentials(
    config: &Ikev2ServerConfig,
    paths: &ResolvedCredentialPaths,
    issuer: &Issuer<'_, KeyPair>,
) -> Result<()> {
    let server_key = generate_ikev2_rsa_key_pair("server")?;
    let server_params = build_server_certificate_params(config)?;
    let server_cert = server_params
        .signed_by(&server_key, issuer)
        .context("generate IKEv2 server certificate failed")?;

    write_string(&paths.server_cert, &server_cert.pem()).await?;
    write_string(
        &paths.server_key,
        &server_key.serialize_pem(),
    )
    .await?;

    Ok(())
}

async fn write_string(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content)
        .await
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn build_ca_issuer(config: &Ikev2ServerConfig) -> Result<(String, String, Issuer<'static, KeyPair>)> {
    let mut ca_params = CertificateParams::new(Vec::new())
        .context("build IKEv2 CA certificate params failed")?;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    ca_params.distinguished_name = build_distinguished_name("Play IKEv2 CA", &config.local_id);

    let ca_key = generate_ikev2_rsa_key_pair("CA")?;
    let ca_cert = ca_params
        .self_signed(&ca_key)
        .context("generate IKEv2 CA certificate failed")?;
    let ca_key_pem = ca_key.serialize_pem();
    let issuer = Issuer::new(ca_params, ca_key);

    Ok((ca_cert.pem(), ca_key_pem, issuer))
}

fn generate_ikev2_rsa_key_pair(purpose: &str) -> Result<KeyPair> {
    KeyPair::generate_rsa_for(&PKCS_RSA_SHA256, RsaKeySize::_2048)
        .with_context(|| format!("generate IKEv2 {purpose} RSA private key failed"))
}

fn build_server_certificate_params(config: &Ikev2ServerConfig) -> Result<CertificateParams> {
    let mut params = CertificateParams::new(Vec::<String>::new())
        .context("build IKEv2 server certificate params failed")?;
    params.subject_alt_names = server_subject_alt_names(config)?;
    params.distinguished_name = build_distinguished_name("Play IKEv2 Server", &config.local_id);
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
        KeyUsagePurpose::KeyAgreement,
    ];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    params.use_authority_key_identifier_extension = true;
    Ok(params)
}

fn build_distinguished_name(common_name_prefix: &str, local_id: &str) -> DistinguishedName {
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(
        DnType::CommonName,
        format!("{common_name_prefix} {}", local_id.trim()),
    );
    distinguished_name
}

fn server_subject_alt_names(config: &Ikev2ServerConfig) -> Result<Vec<SanType>> {
    let mut names = Vec::new();
    let local_id = config.local_id.trim();
    if local_id.is_empty() {
        return Ok(names);
    }

    if let Ok(ip) = local_id.parse::<IpAddr>() {
        names.push(
            SanType::DnsName(
                local_id
                    .try_into()
                    .map_err(|_| anyhow!("invalid IKEv2 local_id `{local_id}` for DNS SAN"))?,
            ),
        );
        names.push(SanType::IpAddress(ip));
    } else {
        names.push(
            SanType::DnsName(
                local_id
                    .try_into()
                    .map_err(|_| anyhow!("invalid IKEv2 local_id `{local_id}` for DNS SAN"))?,
            ),
        );
    }

    Ok(names)
}

fn derive_ca_key_path(ca_cert_path: &Path) -> PathBuf {
    let parent = ca_cert_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let file_name = ca_cert_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("ca-cert.pem");

    let key_name = if let Some(prefix) = file_name.strip_suffix("-cert.pem") {
        format!("{prefix}-key.pem")
    } else if let Some(prefix) = file_name.strip_suffix(".pem") {
        format!("{prefix}-key.pem")
    } else {
        "ca-key.pem".to_string()
    };

    parent.join(key_name)
}

fn render_strongswan_conf(config: &Ikev2ServerConfig, vici_socket_path: &Path) -> String {
    let handshake_level = config.log_level.max(2);
    format!(
        r#"charon {{
  port = {}
  port_nat_t = {}
  filelog {{
    stderr {{
      default = 1
      ike = {handshake_level}
      cfg = {handshake_level}
      net = {handshake_level}
      enc = {handshake_level}
      chd = {handshake_level}
      job = 0
      wch = 0
      lib = 0
      ike_name = yes
      flush_line = yes
    }}
  }}
  plugins {{
    vici {{
      socket = {}
    }}
  }}
}}
"#,
        config.port,
        config.port_nat_t,
        quote_value(&format!("unix://{}", vici_socket_path.display()))
    )
}

fn render_swanctl_conf(
    config: &Ikev2ServerConfig,
    server_cert_name: &str,
    server_key_name: &str,
) -> String {
    let mut text = String::new();
    let child_name = format!("{}-child", config.connection_name);

    text.push_str("connections {\n");
    text.push_str(&format!("  {} {{\n", config.connection_name));
    text.push_str("    version = 2\n");
    text.push_str(&format!(
        "    local_addrs = {}\n",
        render_connection_local_addrs(config)
    ));
    text.push_str("    send_cert = always\n");
    text.push_str(&format!("    mobike = {}\n", yes_no(config.mobike)));
    text.push_str(&format!(
        "    fragmentation = {}\n",
        yes_no(config.fragmentation)
    ));
    text.push_str(&format!(
        "    dpd_delay = {}s\n",
        config.dpd_delay_secs.max(1)
    ));
    if let Some(proposals) = &config.proposals {
        if !proposals.trim().is_empty() {
            text.push_str(&format!("    proposals = {}\n", proposals));
        }
    }
    text.push_str("    pools = play-ipv4\n");
    text.push_str("    local-1 {\n");
    text.push_str("      auth = pubkey\n");
    text.push_str(&format!("      id = {}\n", quote_value(&config.local_id)));
    text.push_str(&format!(
        "      certs = {}\n",
        quote_value(server_cert_name)
    ));
    text.push_str("    }\n");
    text.push_str("    remote-1 {\n");
    text.push_str("      auth = eap-dynamic\n");
    text.push_str("      eap_id = %any\n");
    text.push_str("    }\n");
    text.push_str("    children {\n");
    text.push_str(&format!("      {} {{\n", child_name));
    text.push_str(&format!(
        "        local_ts = {}\n",
        render_list(&config.local_ts)
    ));
    if let Some(esp_proposals) = &config.esp_proposals {
        if !esp_proposals.trim().is_empty() {
            text.push_str(&format!("        esp_proposals = {}\n", esp_proposals));
        }
    }
    text.push_str("      }\n");
    text.push_str("    }\n");
    text.push_str("  }\n");
    text.push_str("}\n\n");

    text.push_str("pools {\n");
    text.push_str("  play-ipv4 {\n");
    text.push_str(&format!("    addrs = {}\n", config.pool));
    if !config.dns_servers.is_empty() {
        text.push_str(&format!("    dns = {}\n", render_list(&config.dns_servers)));
    }
    text.push_str("  }\n");
    text.push_str("}\n\n");

    text.push_str("secrets {\n");
    text.push_str("  private-play {\n");
    text.push_str(&format!(
        "    file = {}\n",
        quote_value(&format!("private/{server_key_name}"))
    ));
    text.push_str("  }\n");
    for (user, password) in &config.eap_users {
        text.push_str(&format!("  eap-{} {{\n", sanitize_section_name(user)));
        text.push_str(&format!("    id = {}\n", quote_value(user)));
        text.push_str(&format!("    secret = {}\n", quote_value(password)));
        text.push_str("  }\n");
    }
    text.push_str("}\n");

    text
}

fn resolve_config_path(path: &str) -> Result<PathBuf> {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        return Ok(candidate);
    }

    let data_dir = std::env::var(DATA_DIR)
        .map(PathBuf::from)
        .map_err(|_| anyhow!("DATA_DIR is not set while resolving `{path}`"))?;
    Ok(data_dir.join(candidate))
}

fn render_list(items: &[String]) -> String {
    items.join(", ")
}

fn quote_value(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn sanitize_section_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn render_connection_local_addrs(config: &Ikev2ServerConfig) -> String {
    let listen_addr = config.listen_addr.trim();
    if listen_addr.is_empty()
        || listen_addr == "0.0.0.0"
        || listen_addr == "::"
        || listen_addr == "[::]"
    {
        "%any".to_string()
    } else {
        listen_addr.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_strongswan_conf_contains_ports_and_socket() {
        let config = Ikev2ServerConfig::default();
        let conf = render_strongswan_conf(&config, Path::new("/tmp/play-ikev2/charon.vici"));
        assert!(conf.contains("port = 500"));
        assert!(conf.contains("port_nat_t = 4500"));
        assert!(conf.contains("unix:///tmp/play-ikev2/charon.vici"));
        assert!(conf.contains("job = 0"));
        assert!(conf.contains("lib = 0"));
        assert!(conf.contains("ike_name = yes"));
    }

    #[test]
    fn render_swanctl_conf_contains_eap_users_and_pool() {
        let mut config = Ikev2ServerConfig::default();
        config.local_id = "vpn.example.com".to_string();
        let conf = render_swanctl_conf(&config, "server-cert.pem", "server-key.pem");
        assert!(conf.contains("auth = eap-dynamic"));
        assert!(conf.contains("addrs = 10.10.10.0/24"));
        assert!(conf.contains("secret = \"change_this_password\""));
        assert!(conf.contains("certs = \"server-cert.pem\""));
        assert!(conf.contains("file = \"private/server-key.pem\""));
        assert!(conf.contains("local_addrs = %any"));
    }

    #[test]
    fn render_connection_local_addrs_uses_wildcard_for_unspecified_listen_addr() {
        let config = Ikev2ServerConfig::default();
        assert_eq!(render_connection_local_addrs(&config), "%any");
    }

    #[tokio::test]
    async fn validate_config_rejects_missing_eap_users() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cert_path = temp_dir.path().join("server-cert.pem");
        let key_path = temp_dir.path().join("server-key.pem");
        fs::write(&cert_path, "cert").await.unwrap();
        fs::write(&key_path, "key").await.unwrap();

        let mut config = Ikev2ServerConfig::default();
        config.server_cert = cert_path.display().to_string();
        config.server_key = key_path.display().to_string();
        config.ca_cert = None;
        config.eap_users.clear();

        let error = validate_config(&config).await.unwrap_err().to_string();
        assert!(error.contains("eap_users"));
    }

    #[tokio::test]
    async fn ensure_credentials_exist_generates_default_bundle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cert_dir = temp_dir.path().join("certs");

        let mut config = Ikev2ServerConfig::default();
        config.server_cert = cert_dir.join("server-cert.pem").display().to_string();
        config.server_key = cert_dir.join("server-key.pem").display().to_string();
        config.ca_cert = Some(cert_dir.join("ca-cert.pem").display().to_string());
        ensure_credentials_exist(&config).await.unwrap();

        let paths = resolved_credential_paths(&config).unwrap();
        assert!(path_exists(&paths.server_cert).await);
        assert!(path_exists(&paths.server_key).await);
        assert!(path_exists(&paths.ca_cert).await);
        assert!(path_exists(&paths.ca_key).await);

        let cert_text = fs::read_to_string(&paths.server_cert).await.unwrap();
        assert!(cert_text.contains("BEGIN CERTIFICATE"));

        validate_config(&config).await.unwrap();
    }

    #[test]
    fn derive_ca_key_path_matches_default_layout() {
        assert_eq!(
            derive_ca_key_path(Path::new("/tmp/certs/ikev2/ca-cert.pem")),
            PathBuf::from("/tmp/certs/ikev2/ca-key.pem")
        );
    }

    #[test]
    fn parse_os_release_detects_debian_bookworm() {
        let info = parse_os_release(
            r#"
ID=debian
VERSION_ID="12"
VERSION_CODENAME=bookworm
"#,
        );

        assert_eq!(
            info,
            OsReleaseInfo {
                id: "debian".to_string(),
                version_id: Some("12".to_string()),
                version_codename: Some("bookworm".to_string()),
            }
        );
    }

    #[test]
    fn generated_ikev2_key_pair_uses_rsa_sha256() {
        let key_pair = generate_ikev2_rsa_key_pair("test").unwrap();
        assert_eq!(key_pair.algorithm(), &PKCS_RSA_SHA256);
    }

    #[test]
    fn ip_local_id_adds_dns_and_ip_subject_alt_names() {
        let mut config = Ikev2ServerConfig::default();
        config.local_id = "203.0.113.10".to_string();

        let names = server_subject_alt_names(&config).unwrap();
        assert_eq!(names.len(), 2);
        assert!(matches!(names[0], SanType::DnsName(_)));
        assert!(matches!(names[1], SanType::IpAddress(_)));
    }

    #[test]
    fn normal_ikev2_daemon_watcher_log_is_not_treated_as_problem() {
        assert!(!looks_like_ikev2_problem(
            "02[JOB] watcher got notification, rebuilding"
        ));
    }

    #[test]
    fn failing_ikev2_daemon_log_is_treated_as_problem() {
        assert!(looks_like_ikev2_problem(
            "00[LIB] failed to load plugin: No such file or directory"
        ));
    }

    #[test]
    fn lookup_executable_in_path_finds_binary_in_custom_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let bin_path = temp_dir.path().join("charon");
        std::fs::write(&bin_path, b"#!/bin/sh\n").unwrap();

        let found = lookup_executable_in_path("charon", Some(temp_dir.path().as_os_str()));
        assert_eq!(found, Some(bin_path));
    }

    #[test]
    fn resolve_executable_with_fallbacks_uses_first_available_fallback() {
        let temp_dir = tempfile::tempdir().unwrap();
        let bin_path = temp_dir.path().join("charon");
        std::fs::write(&bin_path, b"#!/bin/sh\n").unwrap();

        let resolved = resolve_executable_with_fallbacks_in_path(
            "charon-systemd",
            &["charon"],
            Some(temp_dir.path().as_os_str()),
        )
        .unwrap();

        assert_eq!(resolved, bin_path);
    }
}
