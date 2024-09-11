use std::error::Error;
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
use multer::bytes;
use crate::{CheckResponse, ensure};

pub struct ElevenlabsService {
    api_key: String,
    client: Client,
    voice_id: String,
}

impl ElevenlabsService {
    pub fn new(api_key: String,voice_id: String) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(ElevenlabsService {
            api_key,
            client,
            voice_id,
        })
    }

    fn get_headers(&self) -> anyhow::Result<HeaderMap> {
        let api_key = &self.api_key;
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("Accept", HeaderValue::from_str("audio/mpeg")?);
        headers.insert("xi-api-key", HeaderValue::from_str(api_key)?);
        Ok(headers)
    }

    pub async fn text_to_speech(&self, text: &str) -> anyhow::Result<impl futures_core::Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>>> {
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}/stream",self.voice_id);
        // 发起 POST 请求
        let response = self.client.post(url)
            .headers(self.get_headers()?)
            .json(&json!({
                "text": text,
                "model_id": "eleven_monolingual_v1",
                  "voice_settings": {
                    "stability": 0.5,
                    "similarity_boost": 0.5
                  }
            }))
            .send()
            .await?
            .check()
            .await?;

        println!("header : {:?}", response.headers());

        let bytes_stream = response.bytes_stream();
        Ok(bytes_stream)
    }
}


#[cfg(test)]
mod tests {
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;
    use crate::mock_state;
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_all() -> anyhow::Result<()> {
        let s = mock_state!();
        let service = &s.elevenlabs_service;;

        let mut bytes = service.text_to_speech("hello my name is nancy!").await?;
        let mut file = File::create("test.mp3").await?;
        while let Some(chunk) = bytes.next().await {
            let bytes = chunk?;
            file.write_all(&bytes).await?;
        }

        file.flush().await?;
        Ok(())
    }


}
