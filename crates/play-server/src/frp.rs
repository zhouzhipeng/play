use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::Serialize;
use tokio::fs;
#[cfg(feature = "frp-server")]
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tracing::{error, info, warn};

use crate::config::{FrpServerConfig, FrpServerServiceConfig, FrpTransportConfig};

#[cfg(feature = "frp-server")]
#[derive(Serialize)]
struct FrpServerRuntimeDocument {
    server: FrpServerRuntimeConfig,
}

#[cfg(feature = "frp-server")]
#[derive(Serialize)]
struct FrpServerRuntimeConfig {
    bind_addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_token: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    services: BTreeMap<String, FrpServerServiceConfig>,
    #[serde(default)]
    transport: FrpTransportConfig,
    heartbeat_interval: u64,
}

#[cfg(feature = "frp-server")]
impl From<&FrpServerConfig> for FrpServerRuntimeDocument {
    fn from(config: &FrpServerConfig) -> Self {
        Self {
            server: FrpServerRuntimeConfig {
                bind_addr: config.bind_addr.clone(),
                default_token: config.default_token.clone(),
                services: config.services.clone(),
                transport: config.transport.clone(),
                heartbeat_interval: config.heartbeat_interval,
            },
        }
    }
}

pub struct FrpServerHandle {
    #[cfg(feature = "frp-server")]
    shutdown_tx: broadcast::Sender<bool>,
    #[cfg(feature = "frp-server")]
    join_handle: tokio::task::JoinHandle<()>,
    #[cfg(feature = "frp-server")]
    _runtime_dir: tempfile::TempDir,
}

pub struct FrpBackgroundHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: tokio::task::JoinHandle<()>,
}

impl FrpBackgroundHandle {
    pub async fn shutdown(mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Err(error) = self.join_handle.await {
            warn!("FRP background task join failed: {}", error);
        }
    }
}

impl FrpServerHandle {
    pub async fn shutdown(self) {
        #[cfg(feature = "frp-server")]
        {
            let _ = self.shutdown_tx.send(true);
            if let Err(error) = self.join_handle.await {
                warn!("FRP server join failed: {}", error);
            }
        }
    }
}

pub fn maybe_start_frp_server_in_background(
    config: &FrpServerConfig,
) -> Option<FrpBackgroundHandle> {
    if !config.enabled {
        return None;
    }

    let config = config.clone();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    let join_handle = tokio::spawn(async move {
        info!("Starting embedded FRP service in background");

        let startup = maybe_start_frp_server(&config);
        tokio::pin!(startup);

        let handle = tokio::select! {
            result = &mut startup => {
                match result {
                    Ok(handle) => handle,
                    Err(error) => {
                        warn!(
                            "Embedded FRP startup failed in background; HTTP server continues running without FRP: {error:#}"
                        );
                        return;
                    }
                }
            }
            _ = &mut shutdown_rx => {
                info!("Embedded FRP background startup cancelled before completion");
                return;
            }
        };

        let Some(handle) = handle else {
            return;
        };

        let _ = (&mut shutdown_rx).await;
        handle.shutdown().await;
    });

    Some(FrpBackgroundHandle {
        shutdown_tx: Some(shutdown_tx),
        join_handle,
    })
}

pub async fn maybe_start_frp_server(config: &FrpServerConfig) -> Result<Option<FrpServerHandle>> {
    if !config.enabled {
        return Ok(None);
    }

    #[cfg(feature = "frp-server")]
    {
        let (runtime_dir, runtime_config_path) = write_runtime_config(config).await?;

        rathole::Config::from_file(&runtime_config_path)
            .await
            .with_context(|| {
                format!(
                    "invalid embedded FRP server config generated from main config at {}",
                    runtime_config_path.display()
                )
            })?;

        let (shutdown_tx, shutdown_rx) = broadcast::channel(4);
        let args = rathole::Cli {
            config_path: Some(runtime_config_path.clone()),
            server: true,
            client: false,
            genkey: None,
        };

        info!(
            "Starting embedded FRP server from main config using generated runtime config {}",
            runtime_config_path.display()
        );

        let join_handle = tokio::spawn(async move {
            if let Err(error) = rathole::run(args, shutdown_rx).await {
                error!("Embedded FRP server exited with error: {error:#}");
            } else {
                info!("Embedded FRP server stopped");
            }
        });

        return Ok(Some(FrpServerHandle {
            shutdown_tx,
            join_handle,
            _runtime_dir: runtime_dir,
        }));
    }

    #[cfg(not(feature = "frp-server"))]
    {
        warn!(
            "frp_server.enabled is true, but play-server was built without the `frp-server` feature. The FRP server config in main config.toml is ignored"
        );
        Ok(None)
    }
}

#[cfg(feature = "frp-server")]
async fn write_runtime_config(
    config: &FrpServerConfig,
) -> Result<(tempfile::TempDir, std::path::PathBuf)> {
    let runtime_dir = tempfile::tempdir().context("create temporary FRP runtime dir failed")?;
    let runtime_config_path = runtime_dir.path().join("rathole-server.toml");
    let runtime_config = FrpServerRuntimeDocument::from(config);
    let runtime_config_text =
        toml::to_string(&runtime_config).context("serialize embedded FRP server config failed")?;

    if let Some(parent) = runtime_config_path.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("create FRP runtime directory {}", parent.display()))?;
    }

    fs::write(&runtime_config_path, runtime_config_text)
        .await
        .with_context(|| {
            format!(
                "write generated FRP runtime config {}",
                runtime_config_path.display()
            )
        })?;

    Ok((runtime_dir, runtime_config_path))
}

#[cfg(all(test, feature = "frp-server"))]
mod tests {
    use super::*;

    use std::time::Duration;

    use axum::{routing::get, Router};
    use tokio::sync::{broadcast, oneshot};
    use tokio::task::JoinHandle;
    use tokio::time::{sleep, Instant};

    const SMOKE_RESPONSE: &str = "FRP_SMOKE_OK";

    #[tokio::test]
    async fn embedded_frp_server_proxies_http_traffic() -> Result<()> {
        let temp_dir = tempfile::tempdir().context("create temp data dir failed")?;

        let backend_port = free_port()?;
        let tunnel_port = free_port()?;
        let frp_bind_port = free_port()?;

        let client_config_path = temp_dir.path().join("client.toml");

        fs::write(
            &client_config_path,
            format!(
                r#"[client]
remote_addr = "127.0.0.1:{frp_bind_port}"
default_token = "smoke_token"

[client.services.demo_http]
local_addr = "127.0.0.1:{backend_port}"
"#
            ),
        )
        .await
        .with_context(|| format!("write {}", client_config_path.display()))?;

        let (backend_shutdown_tx, backend_handle) = start_backend_server(backend_port).await?;

        let mut services = BTreeMap::new();
        services.insert(
            "demo_http".to_string(),
            FrpServerServiceConfig {
                service_type: crate::config::FrpServiceType::Tcp,
                bind_addr: format!("127.0.0.1:{tunnel_port}"),
                token: None,
                nodelay: None,
            },
        );

        let server_handle = maybe_start_frp_server(&FrpServerConfig {
            enabled: true,
            bind_addr: format!("127.0.0.1:{frp_bind_port}"),
            default_token: Some("smoke_token".to_string()),
            services,
            transport: FrpTransportConfig::default(),
            heartbeat_interval: 30,
        })
        .await?
        .expect("expected embedded FRP server to start");

        let (client_shutdown_tx, client_shutdown_rx) = broadcast::channel(1);
        let client_args = rathole::Cli {
            config_path: Some(client_config_path.clone()),
            server: false,
            client: true,
            genkey: None,
        };

        let client_handle =
            tokio::spawn(async move { rathole::run(client_args, client_shutdown_rx).await });

        let smoke_result = wait_for_tunnel(tunnel_port).await;

        let _ = client_shutdown_tx.send(true);
        let client_result = client_handle.await.context("join FRP client task failed")?;
        server_handle.shutdown().await;
        let _ = backend_shutdown_tx.send(());
        backend_handle.await.context("join backend task failed")??;

        smoke_result?;
        client_result.context("FRP client exited with error")?;
        Ok(())
    }

    async fn start_backend_server(
        port: u16,
    ) -> Result<(oneshot::Sender<()>, JoinHandle<Result<()>>)> {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .with_context(|| format!("bind backend server on {port}"))?;

        let app = Router::new().route("/", get(|| async move { format!("{SMOKE_RESPONSE}\n") }));

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .context("backend server failed")
        });

        Ok((shutdown_tx, handle))
    }

    async fn wait_for_tunnel(tunnel_port: u16) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(500))
            .build()
            .context("build reqwest client failed")?;
        let url = format!("http://127.0.0.1:{tunnel_port}/");
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut last_error = None;

        while Instant::now() < deadline {
            match client.get(&url).send().await {
                Ok(response) => match response.text().await {
                    Ok(body) if body.contains(SMOKE_RESPONSE) => return Ok(()),
                    Ok(body) => {
                        last_error = Some(anyhow::anyhow!("unexpected FRP response body: {body:?}"))
                    }
                    Err(error) => last_error = Some(error.into()),
                },
                Err(error) => last_error = Some(error.into()),
            }

            sleep(Duration::from_millis(250)).await;
        }

        Err(last_error
            .unwrap_or_else(|| anyhow::anyhow!("FRP smoke test timed out without a response")))
    }

    fn free_port() -> Result<u16> {
        let listener =
            std::net::TcpListener::bind(("127.0.0.1", 0)).context("bind ephemeral port failed")?;
        let port = listener
            .local_addr()
            .context("read ephemeral port failed")?
            .port();
        drop(listener);
        Ok(port)
    }
}
