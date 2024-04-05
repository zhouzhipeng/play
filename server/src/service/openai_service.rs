use std::time::Duration;

use http::{HeaderMap, HeaderValue};
use http::header::CONTENT_TYPE;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;
use futures_util::StreamExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use futures_util::TryStreamExt;
use crate::ensure;

pub struct OpenAIService {
    api_key: String,
    client: Client,
    assistant_id: String,
}

#[derive(Deserialize)]
pub struct Thread {
    pub  id: String,
}
#[derive(Serialize)]
pub struct CreateMessage {
    pub role: Role,
    pub content: String,
}
#[derive(Deserialize,Debug )]
pub struct Message {
    content: Vec<MessageContent>,
}
#[derive(Deserialize,Debug )]
pub struct ListMessageResp {
    data: Vec<Message>,
}
#[derive(Deserialize,Debug )]
pub struct MessageContent {
    text: MessageContentValue,
}
#[derive(Deserialize,Debug )]
pub struct MessageContentValue {
    value: String,
}
#[derive(Serialize)]
pub enum  Role {
    user,assistant
}

impl OpenAIService {
    pub fn new(api_key: String,assistant_id: String) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(OpenAIService {
            api_key,
            client,
            assistant_id,
        })
    }
    fn get_headers(&self) -> anyhow::Result<HeaderMap> {
        let api_key = &self.api_key;
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("Authorization", HeaderValue::from_str(&format!("Bearer {}", api_key))?);
        headers.insert("OpenAI-Beta", HeaderValue::from_str("assistants=v1")?);
        Ok(headers)
    }

    pub async fn create_thread(&self) -> anyhow::Result<Thread> {
        // 对话 API 端点
        let url = "https://api.openai.com/v1/threads";
        // 发起 POST 请求
        let response = self.client.post(url)
            .headers(self.get_headers()?)
            .send()
            .await?;
        // 解析响应
        ensure!(response.status().is_success());

        let thread = response.json::<Thread>().await?;
        Ok(thread)
    }
    pub async fn create_message(&self, thread_id: &str , msg: &CreateMessage) -> anyhow::Result<()> {
        // 对话 API 端点
        let url = format!("https://api.openai.com/v1/threads/{}/messages", thread_id);
        // 发起 POST 请求
        let response = self.client.post(url)
            .headers(self.get_headers()?)
            .json(msg)
            .send()
            .await?;
        // 解析响应
        ensure!(response.status().is_success());

        let resp = response.text().await?;
        info!("create message ok , resp :{}", resp);
        Ok(())
    }
    pub async fn list_messages(&self, thread_id: &str) -> anyhow::Result<Vec<Message>> {
        // 对话 API 端点
        let url = format!("https://api.openai.com/v1/threads/{}/messages", thread_id);
        // 发起 POST 请求
        let response = self.client.get(url)
            .headers(self.get_headers()?)
            .send()
            .await?;
        // 解析响应
        ensure!(response.status().is_success());

        let resp = response.json::<ListMessageResp>().await?;
        Ok(resp.data)
    }
    pub async fn run_thread_and_wait(&self, thread_id: &str) -> anyhow::Result<String> {
        // 对话 API 端点
        let url = format!("https://api.openai.com/v1/threads/{}/runs", thread_id);
        // 请求体，根据需要调整 prompt 和 model
        let body = json!({
            "assistant_id": self.assistant_id,
            "stream": true
        });

        // 发起 POST 请求
        let response = self.client.post(url)
            .headers(self.get_headers()?)
            .json(&body)
            .send()
            .await?;
        // 解析响应
        ensure!(response.status().is_success());

        let body = response.bytes_stream();
        let reader = BufReader::new(tokio_util::io::StreamReader::new(body.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))));
        let mut lines = reader.lines();

        let mut result_msg = "".to_string();
        let mut current_event="".to_string();
        let target_event = "thread.message.completed";
        while let Some(line) = lines.next_line().await? {
            if line.starts_with("event:") {
                current_event=line.trim_start_matches("event: ").trim().to_string();
                println!("Event: {}",current_event );

            } else if line.starts_with("data:") {
                let data = line.trim_start_matches("data: ").trim().to_string();
                println!("Data: {}", data);

                if current_event == target_event{
                    let data_json = serde_json::from_str::<Message>(&data)?;
                    result_msg = data_json.content[0].text.value.to_string();
                    break;
                }

            }



        }

        Ok(result_msg)
    }
}


#[cfg(test)]
mod tests {
    use crate::mock_state;
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_all() -> anyhow::Result<()> {
        let s = mock_state!();
        let openai_service = &s.openai_service;;
        let thread  = openai_service.create_thread().await?;
        let msg = CreateMessage{
            role: Role::user,
            content: "hello, who are u?".to_string(),
        };
        openai_service.create_message(&thread.id, &msg).await?;

        let result_msg = openai_service.run_thread_and_wait(&thread.id).await?;
        println!("result msg : {}", result_msg);

        //list messages
        let messsages = openai_service.list_messages(&thread.id).await?;
        println!("messages : {:?}", messsages);

        Ok(())
    }


}
