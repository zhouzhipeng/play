
use anyhow::bail;
use async_channel::{Sender};
use serde_json::Value;
use tracing::error;

use crate::controller::Template;

pub struct TemplateData {
    pub template: Template,
    pub args: Value,
    pub response: Sender<String>
}

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
                error!("receiv error : {:?}", e.to_string());
                bail!("receiv error!")
            }
        }
    }
}
