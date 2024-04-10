use tokio::process::Command;

use crate::{data_dir, method_router};
use crate::R;

method_router!(
    post : "/shell/execute"-> execute_command,
);


async fn execute_command(input: String) -> R<String> {
    let input_tmp = input.trim().to_lowercase();
    if  input_tmp.starts_with("vi")
        || input_tmp.starts_with("top")
        || input_tmp.starts_with("less")
        || input_tmp.starts_with("nano")
        || input_tmp.starts_with("screen")
        || input_tmp.starts_with("tmux")
        || input_tmp.starts_with("ncurses")
        || input_tmp.starts_with("ssh")
        || input_tmp.starts_with("ftp")
        || input_tmp.starts_with("mysql")
    {
        return Ok("command not supported!".to_string())
    }

    let output = Command::new("sh")
        .current_dir(data_dir!())
        .arg("-c")
        .arg(&input)
        .output()
        .await?;

    let result = String::from_utf8_lossy(&output.stdout).to_string();
    let result = result + &String::from_utf8_lossy(&output.stderr).to_string();
    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_execute_command() {
        let r = execute_command("ls -l".to_string()).await;
        println!("{:?}", r);
    }
}

