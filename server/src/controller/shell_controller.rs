use std::convert::Infallible;
use std::process::Stdio;
use std::time::Duration;
use axum::extract::Query;
use axum::response::{IntoResponse, Sse};
use axum::response::sse::Event;
use futures_core::Stream;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::sync::mpsc::unbounded_channel;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{error, info};
use crate::{data_dir, hex_to_string, method_router, return_error, string_to_hex};
use crate::R;
use futures::{stream, StreamExt};
use sqlx::__rt::timeout;  // 引入所需的 futures 库部分

method_router!(
    get : "/shell/execute"-> execute_command,
);

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

