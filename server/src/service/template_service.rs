use crossbeam_channel::{Receiver, Sender};
use serde_json::Value;

pub struct TemplateData {
    pub template: &'static str,
    pub filename: &'static str,
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
    pub fn render_template(&self, filename: &'static str,template: &'static str,  args: Value) -> anyhow::Result<String> {
        self.req_sender.send(TemplateData {
            template,
            filename,
            args,
        })?;
        Ok(self.res_receiver.recv()?)
    }
}
