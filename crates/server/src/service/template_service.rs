
use anyhow::bail;
use async_channel::{Receiver, RecvError, Sender};
use async_trait::async_trait;
use serde_json::Value;
use tracing::error;
use play_shared::tpl_engine_api::{TemplateData, TplEngineAPI};

use crate::Template;
use tokio::time::{self, Duration};


pub struct TemplateService {
    req_sender: Sender<TemplateData>,
}

impl TemplateService {
    pub fn new(req_sender: Sender<TemplateData>) -> Self {
        Self {
            req_sender,
        }
    }

    pub async fn render_template(&self, t: Template, data: Value) -> anyhow::Result<String> {
        let (sender, receiver ) = async_channel::bounded::<String>(1);
        self.req_sender.send(TemplateData {
            template:t,
            args: data,
            response: sender,
        }).await?;

        // Set a timeout for receiving a message
        let timeout_duration = Duration::from_secs(5);
        let result = time::timeout(timeout_duration, receiver.recv()).await;

        return match result {
            Ok(Ok(msg)) => {
                if msg.starts_with("[ERROR]"){
                    bail!(msg)
                }
                Ok(msg)
            },
            Ok(Err(e)) => bail!("Error receiving template message: {}", e),
            Err(_) => bail!("Timeout waiting for template message"),
        }
    }
}

