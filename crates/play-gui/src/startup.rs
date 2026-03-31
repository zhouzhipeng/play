use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{bail, Context, Result};
use directories::BaseDirs;

const MACOS_LAUNCH_AGENT_LABEL: &str = "com.zhouzhipeng.play-gui";
const WINDOWS_RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const WINDOWS_RUN_VALUE: &str = "Play GUI";

pub fn is_supported() -> bool {
    cfg!(target_os = "macos") || cfg!(target_os = "windows")
}

pub fn is_enabled(app_executable: &Path) -> Result<bool> {
    #[cfg(target_os = "macos")]
    {
        let launch_agent_path = macos_launch_agent_path()?;
        if !launch_agent_path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(&launch_agent_path)
            .with_context(|| format!("read {}", launch_agent_path.display()))?;
        let expected_path = xml_escape(&app_executable.display().to_string());
        return Ok(content.contains(&expected_path));
    }

    #[cfg(target_os = "windows")]
    {
        let output = reg_command(["query", WINDOWS_RUN_KEY, "/v", WINDOWS_RUN_VALUE])?;
        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let expected_path = app_executable.display().to_string().replace('/', "\\");
        return Ok(stdout.contains(&expected_path));
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app_executable;
        Ok(false)
    }
}

pub fn set_enabled(app_executable: &Path, enabled: bool) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let launch_agent_path = macos_launch_agent_path()?;

        if enabled {
            if let Some(parent) = launch_agent_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create {}", parent.display()))?;
            }

            fs::write(&launch_agent_path, macos_launch_agent_plist(app_executable))
                .with_context(|| format!("write {}", launch_agent_path.display()))?;
        } else if launch_agent_path.exists() {
            fs::remove_file(&launch_agent_path)
                .with_context(|| format!("remove {}", launch_agent_path.display()))?;
        }

        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        if enabled {
            let value = format!("\"{}\"", app_executable.display().to_string().replace('/', "\\"));
            let output = reg_command([
                "add",
                WINDOWS_RUN_KEY,
                "/v",
                WINDOWS_RUN_VALUE,
                "/t",
                "REG_SZ",
                "/d",
                &value,
                "/f",
            ])?;
            ensure_success(output, "register Play GUI for Windows sign-in")?;
        } else {
            let query = reg_command(["query", WINDOWS_RUN_KEY, "/v", WINDOWS_RUN_VALUE])?;
            if !query.status.success() {
                return Ok(());
            }

            let output = reg_command(["delete", WINDOWS_RUN_KEY, "/v", WINDOWS_RUN_VALUE, "/f"])?;
            ensure_success(output, "remove Play GUI from Windows sign-in")?;
        }

        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app_executable;
        let _ = enabled;
        bail!("startup registration is only supported on macOS and Windows");
    }
}

#[cfg(target_os = "macos")]
fn macos_launch_agent_path() -> Result<PathBuf> {
    let base_dirs = BaseDirs::new().context("resolve home directory failed")?;
    Ok(base_dirs
        .home_dir()
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{MACOS_LAUNCH_AGENT_LABEL}.plist")))
}

#[cfg(target_os = "macos")]
fn macos_launch_agent_plist(app_executable: &Path) -> String {
    let executable = xml_escape(&app_executable.display().to_string());
    let working_directory = app_executable
        .parent()
        .map(|path| xml_escape(&path.display().to_string()))
        .unwrap_or_default();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{MACOS_LAUNCH_AGENT_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{executable}</string>
    </array>
    <key>ProcessType</key>
    <string>Interactive</string>
    <key>RunAtLoad</key>
    <true/>
    <key>WorkingDirectory</key>
    <string>{working_directory}</string>
</dict>
</plist>
"#
    )
}

#[cfg(target_os = "windows")]
fn reg_command<const N: usize>(args: [&str; N]) -> Result<Output> {
    Command::new("reg")
        .args(args)
        .output()
        .with_context(|| format!("run reg {}", args.join(" ")))
}

#[cfg(target_os = "windows")]
fn ensure_success(output: Output, action: &str) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }

    bail!(
        "{action} failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

#[cfg(target_os = "macos")]
fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
