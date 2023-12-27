
use anyhow::bail;
use async_channel::{Receiver, RecvError, Sender};
use async_trait::async_trait;
use serde_json::Value;
use tracing::error;
use shared::tpl_engine_api::{TemplateData, TplEngineAPI};

use crate::Template;


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
        return match receiver.recv().await {
            Ok(s) => Ok(s),
            Err(e) => {
                error!("receive error : {:?}", e.to_string());
                bail!("receive error!")
            }
        }
    }
}

