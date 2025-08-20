use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc as std_mpsc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::{websocket::TerminalResponse, Error, Result};

pub struct LocalTerminal {
    input_tx: Option<std_mpsc::Sender<TerminalCommand>>,
    terminal_task: Option<JoinHandle<()>>,
}

enum TerminalCommand {
    Input(String),
    Resize { cols: u16, rows: u16 },
    Disconnect,
}

impl LocalTerminal {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            input_tx: None,
            terminal_task: None,
        })
    }

    pub async fn start(&mut self, tx: mpsc::Sender<TerminalResponse>) {
        // Use std::sync::mpsc for cross-thread communication
        let (input_tx, input_rx) = std_mpsc::channel::<TerminalCommand>();
        self.input_tx = Some(input_tx);
        
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
            let mut cmd = CommandBuilder::new(&shell);
            cmd.arg("-l"); // Login shell
            
            // Set working directory to DATA_DIR if available
            if let Ok(data_dir) = std::env::var("DATA_DIR") {
                info!("Setting working directory to DATA_DIR: {}", data_dir);
                cmd.cwd(&data_dir);
                // Also set PWD environment variable to help shells recognize the working directory
                cmd.env("PWD", &data_dir);
            } else {
                info!("DATA_DIR not set, using default working directory");
            }
            
            // Set environment variables
            cmd.env("TERM", "xterm-256color");
            cmd.env("COLORTERM", "truecolor");
            
            // Spawn the shell
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
            
            info!("Local terminal started with shell: {}", shell);
            
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
            
            // Send initial connected message
            let _ = rt.block_on(tx.send(TerminalResponse::Connected));
            
            // Create a channel for output thread to signal when it's done
            let (output_done_tx, output_done_rx) = std_mpsc::channel::<()>();
            let tx_clone = tx.clone();
            let rt_clone = rt.clone();
            
            // Spawn a thread to read output from PTY
            std::thread::spawn(move || {
                let mut buffer = vec![0u8; 4096];
                
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => {
                            // EOF - shell might have exited
                            info!("EOF from PTY");
                            break;
                        }
                        Ok(n) => {
                            let data = String::from_utf8_lossy(&buffer[..n]).to_string();
                            info!("Read {} bytes from PTY", n);
                            if rt_clone.block_on(tx_clone.send(TerminalResponse::Output { data })).is_err() {
                                error!("Failed to send output to websocket");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Read error: {}", e);
                            break;
                        }
                    }
                }
                
                let _ = output_done_tx.send(());
            });
            
            // Main loop for handling input commands
            loop {
                // Check for input commands - blocking receive with timeout
                match input_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(TerminalCommand::Input(data)) => {
                        // Write to the master PTY
                        info!("Received input command: {:?}", data);
                        match writer.write_all(data.as_bytes()) {
                            Ok(_) => {
                                match writer.flush() {
                                    Ok(_) => info!("Successfully wrote to PTY"),
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
                        info!("Disconnect command received");
                        break;
                    }
                    Err(std_mpsc::RecvTimeoutError::Timeout) => {
                        // No commands, continue
                    }
                    Err(std_mpsc::RecvTimeoutError::Disconnected) => {
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
                
                // Check if output thread has finished
                if output_done_rx.try_recv().is_ok() {
                    info!("Output thread finished");
                    break;
                }
            }
            
            info!("Closing local terminal");
            let _ = child.kill();
        });

        self.terminal_task = Some(handle);
    }

    pub async fn send_input(&mut self, data: &str) -> Result<()> {
        info!("send_input called with: {:?}", data);
        if let Some(ref tx) = self.input_tx {
            tx.send(TerminalCommand::Input(data.to_string()))
                .map_err(|e| Error::Terminal(format!("Failed to send input: {}", e)))?;
            info!("Input command sent to channel");
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