//! Terminal session handling.

use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{Duration, timeout};

use crate::error::{Error, Result};

/// A message from the terminal to the client.
#[derive(Debug, serde::Serialize)]
pub enum TerminalMessage {
    /// Output from the terminal.
    Output { data: String },
    /// The terminal has exited.
    Exit { code: Option<i32> },
    /// An error has occurred.
    Error { message: String },
}

/// A message from the client to the terminal.
#[derive(Debug, serde::Deserialize)]
pub enum ClientMessage {
    /// Input to send to the terminal.
    Input { data: String },
    /// Resize the terminal.
    Resize { cols: u16, rows: u16 },
    /// Terminate the terminal.
    Terminate,
}

/// A terminal session.
pub struct TerminalSession {
    /// The terminal process.
    process: Child,
    /// The terminal's stdin.
    stdin: ChildStdin,
    /// The terminal's stdout.
    stdout: Option<ChildStdout>,
    /// The channel to send messages to the client.
    tx: mpsc::Sender<TerminalMessage>,
    /// Whether the terminal is running.
    is_running: Arc<Mutex<bool>>,
}

impl TerminalSession {
    /// Creates a new terminal session.
    pub async fn new(
        shell: &str,
        cols: u16,
        rows: u16,
        tx: mpsc::Sender<TerminalMessage>,
    ) -> Result<Self> {
        // Determine the shell command based on the operating system
        let (shell_cmd, shell_args) = if cfg!(target_os = "windows") {
            ("cmd.exe", vec![])
        } else {
            (shell, vec!["-l"])
        };

        // Set environment variables for the pty size
        let mut command = Command::new(shell_cmd);
        command
            .args(shell_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("TERM", "xterm-256color")
            .env("COLUMNS", cols.to_string())
            .env("LINES", rows.to_string());

        // Spawn the process
        let mut process = tokio::process::Command::from(command)
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| Error::Process(format!("Failed to spawn process: {}", e)))?;

        // Get stdin and stdout
        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| Error::Process("Failed to open stdin".into()))?;

        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| Error::Process("Failed to open stdout".into()))?;

        // Create the terminal session
        let session = Self {
            process,
            stdin,
            stdout: Some(stdout),
            tx,
            is_running: Arc::new(Mutex::new(true)),
        };

        Ok(session)
    }

    /// Starts the terminal session.
    pub async fn start(&mut self) {
        let is_running = self.is_running.clone();
        let tx = self.tx.clone();
        let mut stdout = self.stdout.take().expect("stdout should be available");
        let process_clone = self.process.id();

        // Spawn a task to read from stdout
        tokio::spawn(async move {
            let mut buffer = [0u8; 4096];

            loop {
                match timeout(Duration::from_secs(1), stdout.read(&mut buffer)).await {
                    Ok(Ok(0)) => {
                        // EOF, the process has terminated
                        break;
                    }
                    Ok(Ok(n)) => {
                        // Send the output to the client
                        let output = String::from_utf8_lossy(&buffer[..n]).to_string();
                        if tx
                            .send(TerminalMessage::Output { data: output })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(Err(e)) => {
                        // Error reading from stdout
                        let _ = tx
                            .send(TerminalMessage::Error {
                                message: format!("Failed to read from stdout: {}", e),
                            })
                            .await;
                        break;
                    }
                    Err(_) => {
                        // Timeout, check if we should still be running
                        if !*is_running.lock().await {
                            break;
                        }
                    }
                }
            }

            // Mark the terminal as not running
            *is_running.lock().await = false;

            // Send exit message
            let _ = tx.send(TerminalMessage::Exit { code: None }).await;
        });

        // Spawn a separate task to handle process exit
        let tx_clone = self.tx.clone();
        let is_running_clone = self.is_running.clone();
        tokio::spawn(async move {
            // Create a process exit handler
            if let Some(pid) = process_clone {
                // Wait for process to exit
                tokio::time::sleep(Duration::from_secs(1)).await;

                // Send exit code if needed - in a real implementation we would get the actual exit code
                let _ = tx_clone.send(TerminalMessage::Exit { code: Some(0) }).await;

                // Mark the terminal as not running
                *is_running_clone.lock().await = false;
            }
        });
    }

    /// Sends input to the terminal.
    pub async fn send_input(&mut self, data: String) -> Result<()> {
        // 记录收到的输入以便调试
        println!("Sending input to terminal: {:?}", data);

        // 确保输入被正确写入
        self.stdin
            .write_all(data.as_bytes())
            .await
            .map_err(|e| Error::Terminal(format!("Failed to write to stdin: {}", e)))?;

        // 重要：刷新 stdin，确保数据被立即发送
        self.stdin.flush().await
            .map_err(|e| Error::Terminal(format!("Failed to flush stdin: {}", e)))?;

        Ok(())
    }

    /// Resizes the terminal.
    pub async fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        // On Unix systems, we would use the ioctl TIOCSWINSZ call to resize the pty.
        // For this example, we'll just update the environment variables.
        // In a real implementation, you might want to use a crate like pty-process
        // that handles pty resizing properly.

        Ok(())
    }

    /// Terminates the terminal.
    pub async fn terminate(&mut self) -> Result<()> {
        // Mark the terminal as not running
        *self.is_running.lock().await = false;

        // Kill the process
        self.process
            .kill()
            .await
            .map_err(|e| Error::Terminal(format!("Failed to kill process: {}", e)))?;

        Ok(())
    }

    /// Waits for the process to exit and returns the exit code.
    pub async fn wait_for_exit(&mut self) -> Option<i32> {
        match self.process.wait().await {
            Ok(status) => status.code(),
            Err(_) => None,
        }
    }

    /// Returns whether the terminal is running.
    pub async fn is_running(&self) -> bool {
        *self.is_running.lock().await
    }
}