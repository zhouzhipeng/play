use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use play::controller::function_controller::ChatAIReq;
use play::{hex_to_string, mock_server};

#[tokio::test]
async fn test_chat_ai_with_audio() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert json
    let resp = server.post("/functions/chat-ai")
        .form(&ChatAIReq{ input: "any movies to recommend?".to_string() })
        .await;
    // println!("resp text : {}", resp.text());
    resp.assert_status_ok();
    let header = resp.header("x-resp-msg");
    println!("header :  {:?}", hex_to_string!(header.to_str()?));
    let bytes = resp.as_bytes();

    let mut file = File::create("test.mp3").await?;
    file.write_all(bytes).await?;
    Ok(())
}
#[tokio::test]
async fn test_chat_ai_without_audio() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert json
    let resp = server.post("/functions/chat-ai")
        .add_query_param("no_audio", true)
        .form(&ChatAIReq{ input: "hi".to_string() })
        .await;

    resp.assert_status_ok();
    let text = resp.text();
    println!("resp text : {}", text);
    Ok(())
}
