use serde_json::json;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use play::controller::chat_controller::{CAT_GENERAL_OPENAI_THREADS, ChatAIReq, ChatMessageDo, ChatThreadDo};
use play::{hex_to_string, mock_server, mock_server_state, mock_state};
use play::tables::general_data::GeneralData;

#[tokio::test]
async fn test_chat_ai_with_audio() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert json
    let resp = server.post("/chat/submit")
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
    let resp = server.post("/chat/submit")
        .add_query_param("no_audio", true)
        .form(&ChatAIReq{ input: "hi".to_string() })
        .await;

    resp.assert_status_ok();
    let text = resp.text();
    println!("resp text : {}", text);
    Ok(())
}
#[tokio::test]
async fn test_general_chat_ai() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert json
    let resp = server.post("/chat/submit")
        .add_query_param("is_general", true)
        .form(&ChatAIReq{ input: "hi".to_string() })
        .await;

    resp.assert_status_ok();
    let text = resp.text();
    println!("resp text : {}", text);
    Ok(())
}
#[tokio::test]
async fn test_general_chat_ai_in_thread() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert json
    let resp = server.post("/chat/submit")
        .add_query_param("is_general", true)
        .add_query_param("thread_id", "thread_zWvo2L7d3viCEJhC69JpNhwa")
        .form(&ChatAIReq{ input: "i like blue".to_string() })
        .await;
    let text = resp.text();
    println!("resp text : {}", text);
    resp.assert_status_ok();

    Ok(())
}
#[tokio::test]
async fn test_list_threads() -> anyhow::Result<()> {
    let (server, s) = mock_server_state!();
    GeneralData::insert(&GeneralData{
        cat: CAT_GENERAL_OPENAI_THREADS.to_string(),
        data: json!({
            "thread_id" : "thread_111",
            "title": "abc"
        }).to_string(),
      ..GeneralData::default()
    }, &s.db).await?;
    GeneralData::insert(&GeneralData{
        cat: CAT_GENERAL_OPENAI_THREADS.to_string(),
        data: json!({
            "thread_id" : "thread_222",
            "title": "abc222"
        }).to_string(),
      ..GeneralData::default()
    }, &s.db).await?;

    //insert json
    let resp = server.get("/chat/threads")
        .await;
    // let text = resp.json::<Vec<ChatThreadDo>>();
    let text = resp.text();
    println!("resp data : {}", text);
    resp.assert_status_ok();

    Ok(())
}
#[tokio::test]
async fn test_list_message() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert json
    let resp = server.get("/chat/messages")
        .add_query_param("thread_id", "thread_2qzEju9lH7mhxmjubDoNPngA")
        .await;
    let text = resp.json::<Vec<ChatMessageDo>>();
    println!("resp data : {:?}", text);
    resp.assert_status_ok();

    Ok(())
}
