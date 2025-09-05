use std::convert::Infallible;
use std::process::Stdio;
use std::time::Duration;
use axum::extract::Query;
use axum::Json;
use axum::response::{IntoResponse, Sse};
use axum::response::sse::Event;
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::sync::mpsc::unbounded_channel;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{error, info};
use crate::{data_dir, hex_to_string, method_router, return_error, string_to_hex, S};
use crate::R;
use futures::{stream, StreamExt};
use serde_json::{Map, Value};
use sqlx::__rt::timeout;
use crate::tables::general_data::GeneralData;
// 引入所需的 futures 库部分

method_router!(
    get : "/shell/execute"-> execute_command,
    post : "/crontab/apply"-> handle_apply_crontab,
    get : "/crontab/current"-> handle_current_crontab,

);


#[derive(Serialize)]
struct CurrentCrontabResponse {
    success: bool,
    content: String,
    message: Option<String>,
}

async fn handle_current_crontab() -> R<Json<CurrentCrontabResponse>> {
    // Execute the crontab -l command to get current crontab
    let output = Command::new("sh")
        .arg("-c")
        .arg("crontab -l")
        .output()
        .await?;

    if output.status.success() {
        let content = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(Json(CurrentCrontabResponse {
            success: true,
            content,
            message: None,
        }))
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr).to_string();
        // Check if it's the "no crontab for user" message, which is common and not actually an error
        if error_message.contains("no crontab for") {
            Ok(Json(CurrentCrontabResponse {
                success: true,
                content: "".to_string(),
                message: Some("No crontab currently exists for this user.".to_string()),
            }))
        } else {
            Ok(Json(CurrentCrontabResponse {
                success: false,
                content: "".to_string(),
                message: Some(format!("Failed to retrieve current crontab: {}", error_message)),
            }))
        }
    }
}
#[derive(Serialize)]
struct ApplyResponse {
    success: bool,
    message: String,
}

async fn handle_apply_crontab(s: S) -> R<Json<ApplyResponse>> {
    // First, get the current system crontab
    let current_output = Command::new("sh")
        .arg("-c")
        .arg("crontab -l 2>/dev/null || true")
        .output()
        .await?;
    
    let current_crontab = String::from_utf8_lossy(&current_output.stdout);
    
    // Parse existing crontab entries and extract commands
    let mut existing_entries: Vec<String> = Vec::new();
    let mut existing_commands: Vec<String> = Vec::new();
    
    for line in current_crontab.lines() {
        let trimmed = line.trim();
        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            existing_entries.push(line.to_string());
            continue;
        }
        
        // Parse the cron line to extract the command part
        let parts: Vec<&str> = trimmed.splitn(6, ' ').collect();
        if parts.len() == 6 {
            let command = parts[5];
            existing_commands.push(command.to_string());
            existing_entries.push(line.to_string());
        } else {
            // Keep malformed lines as-is
            existing_entries.push(line.to_string());
        }
    }
    
    // Query all crontab entries from database
    let entries = GeneralData::query_composite(
        "*",
        "crontab",
        "0,1000", // Limit: start at index 0, fetch up to 1000 entries
        "1=1",    // Where condition: select all
        false,    // Don't include deleted entries
        "id asc", // Order by ID ascending
        &s.db,
    ).await?;

    // Collect database commands to check for duplicates
    let mut db_commands: Vec<String> = Vec::new();
    let mut db_crontab_lines: Vec<String> = Vec::new();
    
    for entry in &entries {
        let data_map = entry.extract_data()?;

        // Check if entry is enabled (default to true if not specified)
        let enabled = match data_map.get("enabled") {
            Some(Value::Bool(enabled)) => *enabled,
            Some(Value::String(s)) if s == "false" => false,
            Some(Value::Number(n)) if n.as_u64() == Some(0) => false,
            _ => true, // Default to enabled if not specified or if value is unexpected
        };

        // Skip disabled entries
        if !enabled {
            continue;
        }

        // Extract crontab fields with defaults
        let minute = get_value_as_string(&data_map, "minute").unwrap_or_else(|| "*".to_string());
        let hour = get_value_as_string(&data_map, "hour").unwrap_or_else(|| "*".to_string());
        let day_of_month = get_value_as_string(&data_map, "day_of_month").unwrap_or_else(|| "*".to_string());
        let month = get_value_as_string(&data_map, "month").unwrap_or_else(|| "*".to_string());
        let day_of_week = get_value_as_string(&data_map, "day_of_week").unwrap_or_else(|| "*".to_string());
        let command = get_value_as_string(&data_map, "command").unwrap_or_else(|| "".to_string());

        if !command.is_empty() {
            db_commands.push(command.clone());
            db_crontab_lines.push(format!("{} {} {} {} {} {}", minute, hour, day_of_month, month, day_of_week, command));
        }
    }
    
    // Build the final crontab string
    let mut final_crontab = String::new();
    
    // First, add existing entries that don't have matching commands in the database
    for (i, line) in existing_entries.iter().enumerate() {
        let trimmed = line.trim();
        
        // Always keep empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            final_crontab.push_str(line);
            final_crontab.push('\n');
            continue;
        }
        
        // Check if this is a cron entry
        let parts: Vec<&str> = trimmed.splitn(6, ' ').collect();
        if parts.len() == 6 {
            let command = parts[5];
            // Only keep if command is not in database entries
            if !db_commands.contains(&command.to_string()) {
                final_crontab.push_str(line);
                final_crontab.push('\n');
            }
        } else {
            // Keep malformed lines as-is
            final_crontab.push_str(line);
            final_crontab.push('\n');
        }
    }
    
    // Add all database entries (they override matching commands)
    for db_line in db_crontab_lines {
        final_crontab.push_str(&db_line);
        final_crontab.push('\n');
    }

    // Execute a shell command to update the system's crontab
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("echo \"{}\" | crontab -", final_crontab.trim_end()))
        .output()
        .await?;

    if output.status.success() {
        Ok(Json(ApplyResponse {
            success: true,
            message: "Crontab applied successfully".to_string(),
        }))
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(Json(ApplyResponse {
            success: false,
            message: format!("Failed to apply crontab: {}", error_message),
        }))
    }
}

// Helper function to extract string values from the JSON map
fn get_value_as_string(map: &Map<String, Value>, key: &str) -> Option<String> {
    map.get(key).map(|v| match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "".to_string(),
        _ => v.to_string().trim_matches('"').to_string(),
    })
}

#[derive(Deserialize, Debug)]
struct ShellInput {
    shell_hex: String,
}

async fn execute_command(Query(req): Query<ShellInput>) -> Sse<impl Stream<Item=Result<Event, Infallible>>> {
    let input = hex_to_string!(&req.shell_hex).trim().to_string();


    let (sender, mut receiver) = mpsc::unbounded_channel();


    tokio::spawn(async move {
        if let Err(e) = check_input(&input){
            sender.send(e.to_string());
            return
        }

        // Setup the command and pipe the stdout
        let mut child = Command::new("sh")
            .current_dir(data_dir!())
            .arg("-c")
            .arg(&input)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // .stderr(tokio::process::Stdio::piped())
            .spawn()
            .expect("failed to execute process");

        let stdout = BufReader::new(child.stdout.take().expect("failed to get stdout"));
        let stderr = BufReader::new(child.stderr.take().expect("failed to get stderr"));

        // 将 Lines 转换为 Stream 并使用 Box::pin 包装
        let stdout_stream = stream::unfold(stdout.lines(), |mut lines| async {
            lines.next_line().await.transpose().map(|line| (line, lines))
        }).boxed();

        let stderr_stream = stream::unfold(stderr.lines(), |mut lines| async {
            lines.next_line().await.transpose().map(|line| (line, lines))
        }).boxed();

        // 合并两个 Stream
        let mut lines = stream::select(stdout_stream, stderr_stream);
        // Process each line as it becomes available
        let duration = Duration::from_secs(5); // Set a 5 second timeout


        while let Ok(line) =  timeout(duration, lines.next()).await {
            match line{
                Some(line)=>{
                    match line {
                        Ok(line) => {
                            // info!("Received: {}", line);
                            if sender.is_closed() {
                                break;
                            }
                            let r = sender.send(line);
                            // info!("sender result : {:?}", r);
                        }
                        Err(e) => {
                            error!("Error reading line : {:?}", e);
                            break;
                        },
                    }
                },
                None=>{
                    error!("error, empty line");
                    break;
                }
            }

        }

        let r = child.kill().await;

        // Wait for the child process to exit
        info!("Process exeucte done , kill status : {:?}", r);
    });

    let stream = UnboundedReceiverStream::new(receiver)
        .map(|data| Ok(Event::default().data(string_to_hex!(data))));

    Sse::new(stream)
}

fn check_input(input: &str) -> anyhow::Result<()> {
    let input_tmp = input.trim().to_lowercase();
    if input_tmp.starts_with("vi")
        || input_tmp.starts_with("less")
        || input_tmp.starts_with("top")
        || input_tmp.starts_with("nano")
        || input_tmp.starts_with("screen")
        || input_tmp.starts_with("tmux")
        || input_tmp.starts_with("ncurses")
        || input_tmp.starts_with("ssh")
        || input_tmp.starts_with("ftp")
        || input_tmp.starts_with("mysql")
    {
        return_error!("command not supported!")
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_execute_command() {
        // let r = execute_command("ls -l".to_string()).await;
        // println!("{:?}", r);
    }
}

