use crate::error::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmuxSession {
    pub id: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub window_count: usize,
    pub attached_clients: usize,
}

#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, TmuxSession>>>,
    tmux_available: bool,
}

impl SessionManager {
    pub fn new() -> Self {
        // Set up persistent tmux socket before checking availability
        Self::setup_persistent_tmux_socket();
        
        let tmux_available = Self::check_tmux_available();
        if !tmux_available {
            warn!("tmux is not available, sessions will not persist across connections");
        } else {
            info!("tmux is available, persistent sessions enabled");
            // Ensure tmux server is running
            Self::ensure_tmux_server();
        }

        let manager = Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            tmux_available,
        };

        if tmux_available {
            manager.sync_existing_sessions();
        }

        manager
    }
    
    fn setup_persistent_tmux_socket() {
        // Use a persistent location for tmux socket (not /tmp which can be cleared)
        if let Ok(home) = std::env::var("DATA_DIR") {
            let socket_dir = PathBuf::from(&home).join(".tmux");
            
            // Create the directory if it doesn't exist
            if let Err(e) = std::fs::create_dir_all(&socket_dir) {
                warn!("Failed to create tmux socket directory: {}", e);
                return;
            }
            
            // Set TMUX_TMPDIR to use our persistent location
            std::env::set_var("TMUX_TMPDIR", socket_dir.to_str().unwrap_or(""));
            debug!("Set TMUX_TMPDIR to: {:?}", socket_dir);
        }
    }
    
    fn ensure_tmux_server() {
        // Get the socket directory path
        let socket_dir = std::env::var("DATA_DIR")
            .or_else(|_| std::env::var("HOME"))
            .map(|dir| PathBuf::from(&dir).join(".tmux"))
            .unwrap_or_else(|_| PathBuf::from("/tmp/.tmux"));
        
        // Ensure socket directory exists with proper permissions
        if let Err(e) = std::fs::create_dir_all(&socket_dir) {
            error!("Failed to create tmux socket directory: {}", e);
            return;
        }
        
        // Set permissions to 700 (rwx------)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) = std::fs::set_permissions(&socket_dir, std::fs::Permissions::from_mode(0o700)) {
                warn!("Failed to set socket directory permissions: {}", e);
            }
        }
        
        let socket_path = socket_dir.to_str().unwrap_or("/tmp/.tmux");
        debug!("Using tmux socket directory: {}", socket_path);
        
        // Start tmux server with explicit socket path
        let output = Command::new("tmux")
            .env("TMUX_TMPDIR", socket_path)
            .args(&["-S", &format!("{}/default", socket_path)])
            .arg("start-server")
            .output();
            
        match output {
            Ok(output) if output.status.success() => {
                info!("tmux server started/verified successfully at {}", socket_path);
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                if !stderr.contains("server already running") && !stdout.contains("server already running") {
                    warn!("tmux server start warning: stderr={}, stdout={}", stderr, stdout);
                } else {
                    debug!("tmux server already running");
                }
            }
            Err(e) => {
                error!("Failed to start tmux server: {}", e);
                return;
            }
        }
        
        // Verify server is working by listing sessions with same socket path
        let verify = Command::new("tmux")
            .env("TMUX_TMPDIR", socket_path)
            .args(&["-S", &format!("{}/default", socket_path)])
            .args(&["list-sessions"])
            .output();
            
        if let Ok(output) = verify {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("no server running") {
                error!("tmux server not running after start attempt at {}", socket_path);
                
                // Try alternative start method
                info!("Attempting alternative tmux server start method...");
                let alt_start = Command::new("tmux")
                    .env("TMUX_TMPDIR", socket_path)
                    .args(&["new-session", "-d", "-s", "temp-init"])
                    .output();
                
                if let Ok(output) = alt_start {
                    if output.status.success() {
                        info!("tmux server started via temp session creation");
                        // Clean up temp session
                        let _ = Command::new("tmux")
                            .env("TMUX_TMPDIR", socket_path)
                            .args(&["kill-session", "-t", "temp-init"])
                            .output();
                    }
                }
            } else if output.status.success() || stderr.contains("no sessions") {
                info!("tmux server verified and running at {}", socket_path);
            }
        }
    }

    fn check_tmux_available() -> bool {
        Command::new("which")
            .arg("tmux")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn sync_existing_sessions(&self) {
        if !self.tmux_available {
            return;
        }

        let output = Command::new("tmux")
            .args(&["list-sessions", "-F", "#{session_name}:#{session_created}:#{session_windows}:#{session_attached}"])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let sessions_str = String::from_utf8_lossy(&output.stdout);
                let mut sessions = self.sessions.lock().unwrap();
                
                for line in sessions_str.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 4 {
                        let session = TmuxSession {
                            id: Uuid::new_v4().to_string(),
                            name: parts[0].to_string(),
                            created_at: chrono::Utc::now(),
                            last_accessed: chrono::Utc::now(),
                            window_count: parts[2].parse().unwrap_or(1),
                            attached_clients: parts[3].parse().unwrap_or(0),
                        };
                        sessions.insert(session.name.clone(), session);
                    }
                }
                
                info!("Synced {} existing tmux sessions", sessions.len());
            }
        }
    }

    pub fn create_session(&self, name: Option<String>) -> Result<TmuxSession, Error> {
        let session_name = name.unwrap_or_else(|| format!("web-terminal-{}", Uuid::new_v4().to_string().split('-').next().unwrap()));
        
        if !self.tmux_available {
            return Err(Error::Custom("tmux is not available".to_string()));
        }

        // Build tmux command with working directory if DATA_DIR is set
        let mut cmd = Command::new("tmux");
        cmd.args(&["new-session", "-d", "-s", &session_name]);
        
        // Set working directory to DATA_DIR if available
        if let Ok(data_dir) = std::env::var("DATA_DIR") {
            cmd.args(&["-c", &data_dir]);
            debug!("Creating tmux session with working directory: {}", data_dir);
        }
        
        let output = cmd.output()
            .map_err(|e| Error::Custom(format!("Failed to create tmux session: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Custom(format!("Failed to create tmux session: {}", error_msg)));
        }

        // Configure session for better multi-client handling
        // aggressive-resize: Makes windows resize to the smallest client viewing them
        let _ = Command::new("tmux")
            .args(&["set-option", "-t", &session_name, "aggressive-resize", "on"])
            .output();
        
        // window-size: Set to smallest to avoid dots when clients have different sizes
        let _ = Command::new("tmux")
            .args(&["set-window-option", "-t", &session_name, "window-size", "smallest"])
            .output();

        let session = TmuxSession {
            id: Uuid::new_v4().to_string(),
            name: session_name.clone(),
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            window_count: 1,
            attached_clients: 0,
        };

        self.sessions.lock().unwrap().insert(session_name, session.clone());
        
        info!("Created new tmux session: {}", session.name);
        Ok(session)
    }

    pub fn list_sessions(&self) -> Vec<TmuxSession> {
        if !self.tmux_available {
            return vec![];
        }

        self.sync_existing_sessions();
        self.sessions.lock().unwrap().values().cloned().collect()
    }

    pub fn get_session(&self, name: &str) -> Option<TmuxSession> {
        self.sessions.lock().unwrap().get(name).cloned()
    }

    pub fn delete_session(&self, name: &str) -> Result<(), Error> {
        if !self.tmux_available {
            return Err(Error::Custom("tmux is not available".to_string()));
        }

        let output = Command::new("tmux")
            .args(&["kill-session", "-t", name])
            .output()
            .map_err(|e| Error::Custom(format!("Failed to delete tmux session: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Custom(format!("Failed to delete tmux session: {}", error_msg)));
        }

        self.sessions.lock().unwrap().remove(name);
        
        info!("Deleted tmux session: {}", name);
        Ok(())
    }

    pub fn attach_to_session(&self, name: &str) -> Result<(), Error> {
        if !self.tmux_available {
            return Err(Error::Custom("tmux is not available".to_string()));
        }

        if let Some(mut session) = self.get_session(name) {
            session.last_accessed = chrono::Utc::now();
            session.attached_clients += 1;
            self.sessions.lock().unwrap().insert(name.to_string(), session);
            Ok(())
        } else {
            Err(Error::Custom(format!("Session '{}' not found", name)))
        }
    }

    pub fn detach_from_session(&self, name: &str) -> Result<(), Error> {
        if !self.tmux_available {
            return Ok(());
        }

        if let Some(mut session) = self.get_session(name) {
            if session.attached_clients > 0 {
                session.attached_clients -= 1;
            }
            self.sessions.lock().unwrap().insert(name.to_string(), session);
            Ok(())
        } else {
            Ok(())
        }
    }

    pub fn is_tmux_available(&self) -> bool {
        self.tmux_available
    }

    pub fn send_command_to_session(&self, session_name: &str, command: &str) -> Result<(), Error> {
        if !self.tmux_available {
            return Err(Error::Custom("tmux is not available".to_string()));
        }

        let output = Command::new("tmux")
            .args(&["send-keys", "-t", session_name, command, "Enter"])
            .output()
            .map_err(|e| Error::Custom(format!("Failed to send command to tmux session: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Custom(format!("Failed to send command to tmux session: {}", error_msg)));
        }

        debug!("Sent command to tmux session {}: {}", session_name, command);
        Ok(())
    }

    pub fn resize_session(&self, session_name: &str, cols: u16, rows: u16) -> Result<(), Error> {
        if !self.tmux_available {
            return Ok(());
        }

        // First try to resize the window
        let output = Command::new("tmux")
            .args(&[
                "resize-window",
                "-t", session_name,
                "-x", &cols.to_string(),
                "-y", &rows.to_string()
            ])
            .output()
            .map_err(|e| Error::Custom(format!("Failed to resize tmux session: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to resize tmux session: {}", error_msg);
        }

        // Force a refresh to clear any dots/artifacts
        let _ = Command::new("tmux")
            .args(&["refresh-client", "-t", session_name])
            .output();

        debug!("Resized tmux session {} to {}x{}", session_name, cols, rows);
        Ok(())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}