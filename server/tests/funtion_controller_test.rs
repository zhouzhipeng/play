use std::time::Duration;
use http_body::Body;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use play::controller::function_controller::ChatAIReq;
use play::mock_server;
use play::tables::general_data::GeneralData;
#[tokio::test]
async fn test_chat_ai() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert json
    let resp = server.post("/functions/chat-ai")
        .form(&ChatAIReq{ input: "who are u?".to_string() })
        .await;
    resp.assert_status_ok();
    println!("resp :  {:?}", resp.header("x-resp-msg"));
    let bytes = resp.as_bytes();

    let mut file = File::create("test.mp3").await?;
    file.write_all(bytes).await?;
    Ok(())
}
