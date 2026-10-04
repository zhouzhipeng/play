use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub id: String,
    pub curl_id: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub status_code: Option<i32>,
    pub headers: String,
    pub body: String,
    pub duration_ms: u64,
    pub error: Option<String>,
}

pub struct Executor {
    sender: mpsc::Sender<ExecutionResult>,
    pub receiver: mpsc::Receiver<ExecutionResult>,
}

impl Executor {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }

    /// Execute a curl command in a background thread
    pub fn execute(&self, curl_id: String, command: String) {
        let sender = self.sender.clone();

        thread::spawn(move || {
            let result = run_curl(&curl_id, &command);
            let _ = sender.send(result);
        });
    }
}

fn run_curl(curl_id: &str, command: &str) -> ExecutionResult {
    let start = Instant::now();
    let id = uuid::Uuid::new_v4().to_string();

    // Clean up the command: remove line continuations and normalize whitespace
    let cleaned = command
        .replace("\\\n", " ")
        .replace("\\\r\n", " ");

    // Extract the actual curl arguments from the command string
    let args = parse_shell_args(&cleaned);

    if args.is_empty() || args[0] != "curl" {
        return ExecutionResult {
            id,
            curl_id: curl_id.to_string(),
            timestamp: Utc::now(),
            status_code: None,
            headers: String::new(),
            body: String::new(),
            duration_ms: start.elapsed().as_millis() as u64,
            error: Some("Command must start with 'curl'".to_string()),
        };
    }

    // Run curl with -i to include response headers, -s for silent mode
    let mut curl_args: Vec<String> = args[1..].to_vec();

    // Add -s (silent) if not present, to suppress progress bar
    if !curl_args.iter().any(|a| a == "-s" || a == "--silent") {
        curl_args.insert(0, "-s".to_string());
    }

    // Add -w to get status code at the end
    // We'll use a separator to split headers+body from status
    let separator = "\n---CURL_HELPER_STATUS---\n";
    curl_args.push("-w".to_string());
    curl_args.push(format!("{separator}%{{http_code}}"));

    // Add --max-time 60 for 60s timeout
    if !curl_args.iter().any(|a| a == "--max-time" || a == "-m" || a == "--connect-timeout") {
        curl_args.push("--max-time".to_string());
        curl_args.push("60".to_string());
    }

    // Add -i to include headers
    if !curl_args.iter().any(|a| a == "-i" || a == "--include") {
        curl_args.insert(0, "-i".to_string());
    }

    let result = Command::new("curl").args(&curl_args).output();

    let duration = start.elapsed().as_millis() as u64;

    match result {
        Ok(output) => {
            let raw_output = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // Split by our separator
            let (response, status_str) = if let Some(sep_pos) = raw_output.rfind("---CURL_HELPER_STATUS---") {
                let resp = raw_output[..sep_pos].to_string();
                let status = raw_output[sep_pos + separator.len() - 1..].trim().to_string();
                (resp, status)
            } else {
                (raw_output, String::new())
            };

            let status_code = status_str.parse::<i32>().ok();

            // Split response into headers and body
            let (headers, body) = split_response(&response);

            let error = if !output.status.success() && !stderr.is_empty() {
                Some(stderr)
            } else {
                None
            };

            ExecutionResult {
                id,
                curl_id: curl_id.to_string(),
                timestamp: Utc::now(),
                status_code,
                headers,
                body,
                duration_ms: duration,
                error,
            }
        }
        Err(e) => ExecutionResult {
            id,
            curl_id: curl_id.to_string(),
            timestamp: Utc::now(),
            status_code: None,
            headers: String::new(),
            body: String::new(),
            duration_ms: duration,
            error: Some(format!("Failed to execute curl: {}", e)),
        },
    }
}

fn split_response(response: &str) -> (String, String) {
    // HTTP response: headers and body separated by \r\n\r\n or \n\n
    // Handle multiple response blocks (e.g., 301 redirect followed by 200)
    let mut remaining = response;

    loop {
        if let Some(pos) = remaining.find("\r\n\r\n") {
            let headers = &remaining[..pos];
            let body = &remaining[pos + 4..];
            // Check if body starts with another HTTP status line (redirect)
            if body.starts_with("HTTP/") {
                remaining = body;
                continue;
            }
            return (headers.to_string(), body.to_string());
        } else if let Some(pos) = remaining.find("\n\n") {
            let headers = &remaining[..pos];
            let body = &remaining[pos + 2..];
            if body.starts_with("HTTP/") {
                remaining = body;
                continue;
            }
            return (headers.to_string(), body.to_string());
        } else {
            return (String::new(), remaining.to_string());
        }
    }
}

/// Simple shell argument parser that handles single and double quotes
fn parse_shell_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Skip whitespace
        while i < len && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }

        let mut arg = String::new();

        while i < len && !chars[i].is_whitespace() {
            if chars[i] == '\'' {
                // Single quoted string
                i += 1;
                while i < len && chars[i] != '\'' {
                    arg.push(chars[i]);
                    i += 1;
                }
                if i < len {
                    i += 1; // skip closing quote
                }
            } else if chars[i] == '"' {
                // Double quoted string
                i += 1;
                while i < len && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < len {
                        i += 1;
                        arg.push(chars[i]);
                    } else {
                        arg.push(chars[i]);
                    }
                    i += 1;
                }
                if i < len {
                    i += 1; // skip closing quote
                }
            } else if chars[i] == '\\' && i + 1 < len {
                i += 1;
                arg.push(chars[i]);
                i += 1;
            } else {
                arg.push(chars[i]);
                i += 1;
            }
        }

        if !arg.is_empty() {
            args.push(arg);
        }
    }

    args
}
