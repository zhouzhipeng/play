use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use play::controller::function_controller::ChatAIReq;
use play::{hex_to_string, mock_server};

#[tokio::test]
async fn test_chat_ai() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert json
    let resp = server.post("/functions/chat-ai")
        .form(&ChatAIReq{ input: "do u have any movies to recommend?".to_string() })
        .await;
    // println!("resp text : {}", resp.text());
    resp.assert_status_ok();
    let resp = resp.header("x-resp-msg");
    println!("resp :  {:?}", hex_to_string!(resp.to_str()?));
    let bytes = resp.as_bytes();

    let mut file = File::create("test.mp3").await?;
    file.write_all(bytes).await?;
    Ok(())
}
