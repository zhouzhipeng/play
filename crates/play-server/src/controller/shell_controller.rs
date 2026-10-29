use crate::tables::general_data::GeneralData;
use crate::R;
use crate::{data_dir, hex_to_string, method_router, return_error, string_to_hex, S};
use axum::extract::Query;
use axum::response::sse::Event;
use axum::response::{IntoResponse, Sse};
use axum::Json;
use futures::{stream, StreamExt};
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::__rt::timeout;
use std::convert::Infallible;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::sync::mpsc::unbounded_channel;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{error, info};
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
                message: Some(format!(
                    "Failed to retrieve current crontab: {}",
                    error_message
                )),
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
    // The manager is the source of truth for this user's crontab. Replacing it
    // also removes entries that were deleted or disabled in the manager.
    let mut entries = Vec::new();
    let mut offset = 0;
    loop {
        let page = GeneralData::query_composite(
            "*", "crontab", &format!("{},1000", offset), "1=1", false, "id asc", &s.db,
        )
        .await?;
        let count = page.len();
        entries.extend(page);
        if count < 1000 {
            break;
        }
        offset += count;
    }

    let mut final_crontab = String::new();
    for entry in &entries {
        let data = entry.extract_data()?;
        if let Some(line) = crontab_line(&data) {
            final_crontab.push_str(&line);
            final_crontab.push('\n');
        }
    }

    // Pass the file through stdin, without interpreting its contents as shell code.
    let mut child = Command::new("crontab")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().expect("piped crontab stdin");
    stdin.write_all(final_crontab.as_bytes()).await?;
    drop(stdin);
    let output = child.wait_with_output().await?;

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

fn crontab_line(data: &Map<String, Value>) -> Option<String> {
    let enabled = match data.get("enabled") {
        Some(Value::Bool(enabled)) => *enabled,
        Some(Value::String(value)) if value == "false" => false,
        Some(Value::Number(value)) if value.as_u64() == Some(0) => false,
        _ => true,
    };
    let command = get_value_as_string(data, "command").unwrap_or_default();
    if !enabled || command.is_empty() {
        return None;
    }
    let fields = ["minute", "hour", "day_of_month", "month", "day_of_week"]
        .map(|key| get_value_as_string(data, key).unwrap_or_else(|| "*".to_string()));
    Some(format!("{} {}", fields.join(" "), command))
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

async fn execute_command(
    Query(req): Query<ShellInput>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let input = hex_to_string!(&req.shell_hex).trim().to_string();

    let (sender, mut receiver) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        if let Err(e) = check_input(&input) {
            sender.send(e.to_string());
            return;
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
            lines
                .next_line()
                .await
                .transpose()
                .map(|line| (line, lines))
        })
        .boxed();

        let stderr_stream = stream::unfold(stderr.lines(), |mut lines| async {
            lines
                .next_line()
                .await
                .transpose()
                .map(|line| (line, lines))
        })
        .boxed();

        // 合并两个 Stream
        let mut lines = stream::select(stdout_stream, stderr_stream);
        // Process each line as it becomes available
        let duration = Duration::from_secs(5); // Set a 5 second timeout

        while let Ok(line) = timeout(duration, lines.next()).await {
            match line {
                Some(line) => {
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
                        }
                    }
                }
                None => {
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

    #[test]
    fn crontab_line_respects_enabled_and_preserves_command() {
        let mut data = serde_json::json!({
            "minute": "0", "hour": "4,12,20", "command": "echo \"$HOME\" $(date)"
        }).as_object().unwrap().clone();
        assert_eq!(crontab_line(&data).as_deref(), Some("0 4,12,20 * * * echo \"$HOME\" $(date)"));
        data.insert("enabled".into(), Value::Bool(false));
        assert_eq!(crontab_line(&data), None);
    }

    #[tokio::test]
    async fn test_execute_command() {
        // let r = execute_command("ls -l".to_string()).await;
        // println!("{:?}", r);
    }
}
