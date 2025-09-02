use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::process::Command;
use std::sync::mpsc as std_mpsc;
use tokio::sync::mpsc;
use mpsc::error;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::{websocket::TerminalResponse, Error, Result};

pub struct LocalTerminal {
    input_tx: Option<std_mpsc::Sender<TerminalCommand>>,
    terminal_task: Option<JoinHandle<()>>,
    session_name: Option<String>,
    use_tmux: bool,
}

enum TerminalCommand {
    Input(String),
    Resize { cols: u16, rows: u16 },
    Disconnect,
}

impl LocalTerminal {
    pub async fn new(session_name: Option<String>) -> Result<Self> {
        let use_tmux = session_name.is_some() && Self::check_tmux_available();
        
        if session_name.is_some() && !use_tmux {
            warn!("tmux session requested but tmux is not available, falling back to regular terminal");
        }
        
        Ok(Self {
            input_tx: None,
            terminal_task: None,
            session_name,
            use_tmux,
        })
    }
    
    fn check_tmux_available() -> bool {
        Command::new("which")
            .arg("tmux")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    pub async fn start(&mut self, tx: mpsc::Sender<TerminalResponse>) {
        // Use std::sync::mpsc for cross-thread communication
        let (input_tx, input_rx) = std_mpsc::channel::<TerminalCommand>();
        self.input_tx = Some(input_tx);
        
        let use_tmux = self.use_tmux;
        let session_name = self.session_name.clone();
        
        let handle = tokio::task::spawn_blocking(move || {
            // Create a new pty
            let pty_system = native_pty_system();
            
            let pair = match pty_system.openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            }) {
                Ok(pair) => pair,
                Err(e) => {
                    error!("Failed to open PTY: {}", e);
                    let rt = tokio::runtime::Handle::current();
                    let _ = rt.block_on(tx.send(TerminalResponse::Error {
                        message: format!("Failed to open terminal: {}", e),
                    }));
                    return;
                }
            };
            
            // Get the shell from environment or use default
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
            let mut cmd = if use_tmux && session_name.is_some() {
                // Use tmux to attach to or create session
                let session = session_name.as_ref().unwrap();
                let mut tmux_cmd = CommandBuilder::new("tmux");
                
                // Check if session exists
                let session_exists = Command::new("tmux")
                    .args(&["has-session", "-t", session])
                    .output()
                    .map(|output| output.status.success())
                    .unwrap_or(false);
                
                if session_exists {
                    debug!("Attaching to existing tmux session: {}", session);
                    tmux_cmd.args(&["attach-session", "-t", session]);
                } else {
                    debug!("Creating new tmux session: {}", session);
                    tmux_cmd.args(&["new-session", "-s", session]);
                }
                
                tmux_cmd
            } else {
                let mut shell_cmd = CommandBuilder::new(&shell);
                
                // Use interactive shell instead of login shell to prevent quick exit
                if shell.contains("bash") {
                    shell_cmd.args(&["-i"]); // Interactive bash
                } else if shell.contains("zsh") {
                    shell_cmd.args(&["-i"]); // Interactive zsh  
                } else if shell.contains("fish") {
                    shell_cmd.args(&["-i"]); // Interactive fish
                } else {
                    // For other shells, try interactive flag
                    shell_cmd.args(&["-i"]);
                }
                
                shell_cmd
            };
            
            // Set working directory to DATA_DIR if available
            if let Ok(data_dir) = std::env::var("DATA_DIR") {
                debug!("Setting working directory to DATA_DIR: {}", data_dir);
                cmd.cwd(&data_dir);
                // Also set PWD environment variable to help shells recognize the working directory
                cmd.env("PWD", &data_dir);
            } else {
                debug!("DATA_DIR not set, using default working directory");
            }
            
            // Set environment variables
            cmd.env("TERM", "xterm-256color");
            cmd.env("COLORTERM", "truecolor");
            
            // Spawn the shell or tmux
            let spawn_msg = if use_tmux && session_name.is_some() {
                format!("tmux session: {}", session_name.as_ref().unwrap())
            } else {
                format!("shell: {}", shell)
            };
            debug!("Attempting to spawn {}", spawn_msg);
            let mut child = match pair.slave.spawn_command(cmd) {
                Ok(child) => child,
                Err(e) => {
                    error!("Failed to spawn shell: {}", e);
                    let rt = tokio::runtime::Handle::current();
                    let _ = rt.block_on(tx.send(TerminalResponse::Error {
                        message: format!("Failed to spawn shell: {}", e),
                    }));
                    return;
                }
            };
            
            debug!("Local terminal started successfully with {}", spawn_msg);
            
            // Get writer for the master PTY
            let mut writer = match pair.master.take_writer() {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed to get writer: {}", e);
                    return;
                }
            };
            
            // Clone reader for output thread
            let mut reader = match pair.master.try_clone_reader() {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to get reader: {}", e);
                    return;
                }
            };
            
            let rt = tokio::runtime::Handle::current();
            
            // Note: Connected message is now sent from websocket.rs with session info
            
            // Create a channel for output thread to signal when it's done
            let (output_done_tx, output_done_rx) = std_mpsc::channel::<()>();
            let tx_clone = tx.clone();
            let rt_clone = rt.clone();
            
            // Spawn a thread to read output from PTY
            std::thread::spawn(move || {
                let mut buffer = vec![0u8; 8192]; // Increase buffer size for better performance
                
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => {
                            // EOF - shell might have exited
                            debug!("EOF from PTY");
                            break;
                        }
                        Ok(n) => {
                            let data = String::from_utf8_lossy(&buffer[..n]).to_string();
                            debug!("Read {} bytes from PTY", n);
                            
                            // Use try_send first to avoid blocking, fallback to blocking send
                            match tx_clone.try_send(TerminalResponse::Output { data: data.clone() }) {
                                Ok(_) => {
                                    // Message sent successfully without blocking
                                }
                                Err(mpsc::error::TrySendError::Full(_)) => {
                                    // Channel is full, use blocking send with timeout
                                    debug!("Channel full, using blocking send...");
                                    if rt_clone.block_on(async {
                                        tokio::time::timeout(
                                            std::time::Duration::from_secs(5),
                                            tx_clone.send(TerminalResponse::Output { data })
                                        ).await
                                    }).is_err() {
                                        error!("Timeout sending output to websocket");
                                        break;
                                    }
                                }
                                Err(mpsc::error::TrySendError::Closed(_)) => {
                                    debug!("WebSocket channel closed");
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Read error: {}", e);
                            break;
                        }
                    }
                    
                    // Add a small delay to prevent overwhelming the WebSocket
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                
                let _ = output_done_tx.send(());
            });
            
            // Main loop for handling input commands
            loop {
                // Check for input commands - blocking receive with timeout
                match input_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(TerminalCommand::Input(data)) => {
                        // Write to the master PTY
                        debug!("Received input command: {:?}", data);
                        match writer.write_all(data.as_bytes()) {
                            Ok(_) => {
                                match writer.flush() {
                                    Ok(_) => debug!("Successfully wrote to PTY"),
                                    Err(e) => error!("Failed to flush: {}", e),
                                }
                            }
                            Err(e) => {
                                error!("Failed to write to terminal: {}", e);
                                let _ = rt.block_on(tx.send(TerminalResponse::Error {
                                    message: format!("Write error: {}", e),
                                }));
                            }
                        }
                    }
                    Ok(TerminalCommand::Resize { cols, rows }) => {
                        if let Err(e) = pair.master.resize(PtySize {
                            rows,
                            cols,
                            pixel_width: 0,
                            pixel_height: 0,
                        }) {
                            error!("Failed to resize PTY: {}", e);
                        }
                    }
                    Ok(TerminalCommand::Disconnect) => {
                        debug!("Disconnect command received");
                        break;
                    }
                    Err(std_mpsc::RecvTimeoutError::Timeout) => {
                        // No commands, continue
                    }
                    Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                        debug!("Command channel disconnected");
                        break;
                    }
                }
                
                // Check if child process is still running
                if let Ok(Some(status)) = child.try_wait() {
                    error!("Shell exited unexpectedly with status: {:?}", status);
                    
                    // Send error message instead of just disconnected
                    let error_msg = if status.success() {
                        "Shell session ended normally".to_string()
                    } else {
                        format!("Shell exited with error code: {:?}", status)
                    };
                    
                    let _ = rt.block_on(tx.send(TerminalResponse::Error {
                        message: error_msg,
                    }));
                    let _ = rt.block_on(tx.send(TerminalResponse::Disconnected));
                    break;
                }
                
                // Check if output thread has finished
                if output_done_rx.try_recv().is_ok() {
                    error!("Output thread finished unexpectedly - shell may have crashed");
                    let _ = rt.block_on(tx.send(TerminalResponse::Error {
                        message: "Terminal output stream ended unexpectedly".to_string(),
                    }));
                    let _ = rt.block_on(tx.send(TerminalResponse::Disconnected));
                    break;
                }
            }
            
            debug!("Closing local terminal");
            let _ = child.kill();
        });

        self.terminal_task = Some(handle);
    }

    pub async fn send_input(&mut self, data: &str) -> Result<()> {
        debug!("send_input called with: {:?}", data);
        if let Some(ref tx) = self.input_tx {
            tx.send(TerminalCommand::Input(data.to_string()))
                .map_err(|e| Error::Terminal(format!("Failed to send input: {}", e)))?;
            debug!("Input command sent to channel");
        } else {
            return Err(Error::Terminal("Terminal not started".to_string()));
        }
        Ok(())
    }

    pub async fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        if let Some(ref tx) = self.input_tx {
            tx.send(TerminalCommand::Resize { cols, rows })
                .map_err(|e| Error::Terminal(format!("Failed to send resize: {}", e)))?;
        }
        Ok(())
    }

    pub async fn disconnect(&mut self) {
        if let Some(ref tx) = self.input_tx {
            let _ = tx.send(TerminalCommand::Disconnect);
        }
        
        if let Some(task) = self.terminal_task.take() {
            let _ = task.await;
        }
    }
}