use std::time::Duration;
use anyhow::Context;

use http::{HeaderMap, HeaderValue};
use http::header::CONTENT_TYPE;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;
use futures_util::StreamExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use futures_util::TryStreamExt;
use tokio::sync::mpsc::UnboundedSender;
use shared::models::check_response;
use crate::{ CheckResponse, ensure};
use crate::types::openai_message::{Message, MessageList};
use crate::types::openai_message_delta::MessageDelta;
use crate::types::openai_runobj::RunObj;

pub struct OpenAIService {
    api_key: String,
    client: Client,
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
pub struct CallGetWeatherArg {
    location: String,
}
#[derive(Serialize)]
pub enum  Role {
    user,assistant
}

impl OpenAIService {
    pub fn new(api_key: String) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;

        Ok(OpenAIService {
            api_key,
            client,
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
            .await?
            .check()
            .await?;
        // 解析响应
        let thread = response.json::<Thread>().await?;
        Ok(thread)
    }

    pub async fn submit_tool_outputs(&self, thread_id: &str, run_id:&str, tool_call_id: &str , output: &str) -> anyhow::Result<String> {
        // 对话 API 端点
        let url = format!("https://api.openai.com/v1/threads/{}/runs/{}/submit_tool_outputs", thread_id,run_id);
        // 发起 POST 请求
        let response = self.client.post(url)
            .headers(self.get_headers()?)
            .json(&json!({
                "tool_outputs": [{
                  "tool_call_id": tool_call_id,
                  "output": output
                }],
                 "stream": true
            }))
            .send()
            .await?
            .check().await?;
        let body = response.bytes_stream();
        let reader = BufReader::new(tokio_util::io::StreamReader::new(body.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))));
        let mut lines = reader.lines();

        let mut result_msg = "".to_string();
        while let Some(line) = lines.next_line().await? {
            if line.starts_with("event:") {
                let next_line = lines.next_line().await?.context("submit_tool_outputs error data!")?;

                let event =line.trim_start_matches("event: ").trim().to_string();
                // println!("Event: {}", event);
                let data = next_line.trim_start_matches("data: ").trim().to_string();
                // println!("Data: {}", data);


                if event == "thread.message.completed"{
                    let data_json = serde_json::from_str::<Message>(&data)?;
                    result_msg = data_json.content.get(0).context("thread.message.completed data error!")?.text.value.to_string();
                    info!("resp msg >> {}", result_msg);
                    break;
                }

            }

        }

        Ok(result_msg)
    }
    pub async fn list_messages(&self, thread_id: &str) -> anyhow::Result<Vec<Message>> {
        // 对话 API 端点
        let url = format!("https://api.openai.com/v1/threads/{}/messages", thread_id);
        // 发起 POST 请求
        let response = self.client.get(url)
            .headers(self.get_headers()?)
            .send()
            .await?
            .check()
            .await?;
        // 解析响应
        let resp = response.json::<MessageList>().await?;
        Ok(resp.data)
    }
    pub async fn run_thread_and_wait(&self, thread_id: &str,assistant_id: &str,  msg: &CreateMessage) -> anyhow::Result<String> {
        // 对话 API 端点
        let url = format!("https://api.openai.com/v1/threads/{}/runs", thread_id);
        // 请求体，根据需要调整 prompt 和 model
        let body = json!({
            "assistant_id": assistant_id,
            "stream": true,
            "additional_messages": [
                msg
            ]
        });

        // 发起 POST 请求
        let response = self.client.post(url)
            .headers(self.get_headers()?)
            .json(&body)
            .send()
            .await?
            .check()
            .await?;


        let body = response.bytes_stream();
        let reader = BufReader::new(tokio_util::io::StreamReader::new(body.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))));
        let mut lines = reader.lines();

        let mut result_msg = "".to_string();
        while let Some(line) = lines.next_line().await? {
            if line.starts_with("event:"){
                let next_line = lines.next_line().await?.context("run_thread_and_wait error data!")?;

                let event =line.trim_start_matches("event: ").trim().to_string();
                // println!("Event: {}", event);
                let data = next_line.trim_start_matches("data: ").trim().to_string();
                // println!("Data: {}", data);


                if event == "thread.message.completed"{
                    let data_json = serde_json::from_str::<Message>(&data)?;
                    result_msg = data_json.content.get(0).context("thread.message.completed data error!")?.text.value.to_string();
                    info!("resp msg >> {}", result_msg);
                    break;
                }else if event == "thread.run.requires_action"{
                    let run_obj = serde_json::from_str::<RunObj>(&data)?;
                    let tool_call = run_obj.required_action.submit_tool_outputs.tool_calls.get(0).context("tool_calls is empty!")?;
                    if tool_call.function.name == "get_weather"{
                        let arg = serde_json::from_str::<CallGetWeatherArg>(&tool_call.function.arguments)?;
                        //todo: call real weather api
                        println!("call get_weather arg : {:?}",  arg);

                        //call submit tool output api
                        result_msg = self.submit_tool_outputs(thread_id, &run_obj.id, &tool_call.id, "30").await?;
                    }
                    break;
                }

            }

        }

        Ok(result_msg)
    }
    pub async fn run_thread_sse(&self, thread_id: &str, assistant_id: &str, msg: &CreateMessage, sender: UnboundedSender<String>) -> anyhow::Result<String> {
        // 对话 API 端点
        let url = format!("https://api.openai.com/v1/threads/{}/runs", thread_id);
        // 请求体，根据需要调整 prompt 和 model
        let body = json!({
            "assistant_id": assistant_id,
            "stream": true,
            "additional_messages": [
                msg
            ]
        });

        // 发起 POST 请求
        let response = self.client.post(url)
            .headers(self.get_headers()?)
            .json(&body)
            .send()
            .await?
            .check()
            .await?;

        sender.send(format!("{}",thread_id));

        let body = response.bytes_stream();
        let reader = BufReader::new(tokio_util::io::StreamReader::new(body.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))));
        let mut lines = reader.lines();

        let mut result_msg = "".to_string();
        while let Some(line) = lines.next_line().await? {
            if line.starts_with("event:"){
                let next_line = lines.next_line().await?.context("run_thread_and_wait error data!")?;

                let event =line.trim_start_matches("event: ").trim().to_string();
                // println!("Event: {}", event);
                let data = next_line.trim_start_matches("data: ").trim().to_string();
                // println!("Data: {}", data);

                if event == "thread.message.delta"{
                    let data_json = serde_json::from_str::<MessageDelta>(&data)?;
                    let delta_msg = data_json.delta.content[0].text.value.to_string();
                    let r = sender.send(delta_msg);
                    // info!("delta sender result : {:?}", r);
                }
                else if event == "thread.message.completed"{
                    let data_json = serde_json::from_str::<Message>(&data)?;
                    result_msg = data_json.content.get(0).context("thread.message.completed data error!")?.text.value.to_string();
                    // info!("resp msg >> {}", result_msg);
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
            content: "hi, what's the weather in dubai now?".to_string(),
        };

        let result_msg = openai_service.run_thread_and_wait(&thread.id , &s.config.open_ai.general_assistant_id, &msg).await?;
        println!("result msg : {}", result_msg);

        Ok(())
    }
    #[ignore]
    #[tokio::test]
    async fn test_context() -> anyhow::Result<()> {
        let aa = vec![1];
        let b = aa.get(1).context("data error")?;
        println!("{}", b);
        Ok(())
    }
    #[ignore]
    #[tokio::test]
    async fn test_list_message() -> anyhow::Result<()> {
        let s = mock_state!();
        let openai_service = &s.openai_service;;
        // //list messages
        let messsages = openai_service.list_messages("thread_2qzEju9lH7mhxmjubDoNPngA").await?;
        println!("messages : {:?}", messsages);

        Ok(())
    }


}
