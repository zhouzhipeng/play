use anyhow::bail;
use async_channel::{Receiver, RecvError, Sender};

use play_shared::tpl_engine_api::TemplateData;
use serde_json::Value;
use tracing::error;

use crate::{render_template_new, Template};
use tokio::time::{self, Duration};

pub struct TemplateService {
    req_sender: Sender<TemplateData>,
}

impl TemplateService {
    pub fn new(req_sender: Sender<TemplateData>) -> Self {
        Self { req_sender }
    }

    pub async fn render_template(&self, t: Template, data: Value) -> anyhow::Result<String> {
        match t {
            Template::StaticTemplate { .. } => bail!("Static template is not supported"),
            Template::DynamicTemplate { content, .. } => render_template_new(&content, data).await,
            Template::PythonCode { .. } => bail!("Python code is not supported"),
        }
    }
}
