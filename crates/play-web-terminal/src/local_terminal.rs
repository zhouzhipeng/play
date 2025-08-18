use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::{websocket::TerminalResponse, Error, Result};

pub struct LocalTerminal {
    input_tx: mpsc::Sender<TerminalCommand>,
    terminal_task: Option<JoinHandle<()>>,
}

enum TerminalCommand {
    Input(String),
    Resize { cols: u16, rows: u16 },
    Disconnect,
}

impl LocalTerminal {
    pub async fn new() -> Result<Self> {
        let (input_tx, mut input_rx) = mpsc::channel::<TerminalCommand>(100);
        
        Ok(Self {
            input_tx,
            terminal_task: None,
        })
    }

    pub async fn start(&mut self, tx: mpsc::Sender<TerminalResponse>) {
        let (new_input_tx, mut input_rx) = mpsc::channel::<TerminalCommand>(100);
        let old_tx = std::mem::replace(&mut self.input_tx, new_input_tx);
        drop(old_tx);
        
        let handle = tokio::task::spawn_blocking(move || {
            // Create a new pty
            let pty_system = native_pty_system();
            
            let pty_pair = match pty_system.openpty(PtySize {
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
            let mut cmd = CommandBuilder::new(&shell);
            cmd.arg("-l"); // Login shell
            
            // Set environment variables
            cmd.env("TERM", "xterm-256color");
            cmd.env("COLORTERM", "truecolor");
            
            // Spawn the shell
            let mut child = match pty_pair.slave.spawn_command(cmd) {
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
            
            info!("Local terminal started with shell: {}", shell);
            
            // Get reader and writer for the master side
            let mut reader = match pty_pair.master.try_clone_reader() {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to clone reader: {}", e);
                    return;
                }
            };
            
            let mut writer = match pty_pair.master.take_writer() {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed to take writer: {}", e);
                    return;
                }
            };
            
            let rt = tokio::runtime::Handle::current();
            let mut buffer = vec![0u8; 4096];
            
            // Send initial connected message
            let _ = rt.block_on(tx.send(TerminalResponse::Connected));
            
            loop {
                // Check for input commands
                match input_rx.try_recv() {
                    Ok(TerminalCommand::Input(data)) => {
                        if let Err(e) = writer.write_all(data.as_bytes()) {
                            error!("Failed to write to terminal: {}", e);
                            let _ = rt.block_on(tx.send(TerminalResponse::Error {
                                message: format!("Write error: {}", e),
                            }));
                        }
                        if let Err(e) = writer.flush() {
                            error!("Failed to flush: {}", e);
                        }
                    }
                    Ok(TerminalCommand::Resize { cols, rows }) => {
                        if let Err(e) = pty_pair.master.resize(PtySize {
                            rows,
                            cols,
                            pixel_width: 0,
                            pixel_height: 0,
                        }) {
                            error!("Failed to resize PTY: {}", e);
                        }
                    }
                    Ok(TerminalCommand::Disconnect) => {
                        info!("Disconnect command received");
                        break;
                    }
                    Err(mpsc::error::TryRecvError::Empty) => {}
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        info!("Command channel disconnected");
                        break;
                    }
                }
                
                // Check if child process is still running
                if let Ok(Some(status)) = child.try_wait() {
                    info!("Shell exited with status: {:?}", status);
                    let _ = rt.block_on(tx.send(TerminalResponse::Disconnected));
                    break;
                }
                
                // Read output from terminal
                match reader.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        let data = String::from_utf8_lossy(&buffer[..n]).to_string();
                        if rt.block_on(tx.send(TerminalResponse::Output { data })).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => {
                        error!("Read error: {}", e);
                        let _ = rt.block_on(tx.send(TerminalResponse::Error {
                            message: format!("Terminal read error: {}", e),
                        }));
                        break;
                    }
                }
            }
            
            info!("Closing local terminal");
            let _ = child.kill();
        });

        self.terminal_task = Some(handle);
    }

    pub async fn send_input(&mut self, data: &str) -> Result<()> {
        self.input_tx
            .send(TerminalCommand::Input(data.to_string()))
            .await
            .map_err(|e| Error::Terminal(format!("Failed to send input: {}", e)))?;
        Ok(())
    }

    pub async fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.input_tx
            .send(TerminalCommand::Resize { cols, rows })
            .await
            .map_err(|e| Error::Terminal(format!("Failed to send resize: {}", e)))?;
        Ok(())
    }

    pub async fn disconnect(&mut self) {
        let _ = self.input_tx.send(TerminalCommand::Disconnect).await;
        
        if let Some(task) = self.terminal_task.take() {
            let _ = task.await;
        }
    }
}