use crate::error::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

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
        let tmux_available = Self::check_tmux_available();
        if !tmux_available {
            warn!("tmux is not available, sessions will not persist across connections");
        } else {
            info!("tmux is available, persistent sessions enabled");
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

        let output = Command::new("tmux")
            .args(&["new-session", "-d", "-s", &session_name])
            .output()
            .map_err(|e| Error::Custom(format!("Failed to create tmux session: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Custom(format!("Failed to create tmux session: {}", error_msg)));
        }

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

        debug!("Resized tmux session {} to {}x{}", session_name, cols, rows);
        Ok(())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}