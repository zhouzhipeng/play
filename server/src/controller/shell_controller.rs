use tokio::process::Command;

use crate::{data_dir, method_router};
use crate::R;

method_router!(
    post : "/shell/execute"-> execute_command,
);


async fn execute_command(input: String) -> R<String> {
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

