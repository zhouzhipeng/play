use async_channel::{Receiver, Sender};
use async_trait::async_trait;
use include_dir::Dir;
use serde_json::Value;

pub struct TemplateData {
    pub template: Template,
    pub args: Value,
    pub response: Sender<String>,
}


pub enum Template {
    StaticTemplate {
        name: &'static str,
        content: &'static str,
    },
    DynamicTemplate {
        name: String,
        content: String,
    },
    PythonCode {
        name: String,
        content: String,
    },
}




#[async_trait]
pub trait TplEngineAPI {

    async fn run_loop(&self, req_receiver: Receiver<TemplateData>);
}