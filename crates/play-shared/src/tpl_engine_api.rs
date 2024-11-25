use async_channel::{Receiver, Sender};


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



