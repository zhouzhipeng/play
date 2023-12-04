use anyhow::{anyhow, bail};
use async_channel::{Receiver, Sender};
use serde_json::Value;
use crate::controller::Template;

pub struct TemplateData {
    pub template: Template,
    pub args: Value,
}

pub struct TemplateService {
    req_sender: Sender<TemplateData>,
    res_receiver: Receiver<String>,
}

impl TemplateService {
    pub fn new(req_sender: Sender<TemplateData>, res_receiver: Receiver<String>) -> Self {
        Self {
            req_sender,
            res_receiver,
        }
    }

    pub async fn render_template(&self, t: Template, data: Value) -> anyhow::Result<String> {
        self.req_sender.send(TemplateData {
            template:t,
            args: data,
        }).await?;
        Ok(self.res_receiver.recv().await?)
    }
}
