use anyhow::anyhow;
use crossbeam_channel::{Receiver, Sender};
use include_dir::{Dir, include_dir};

use serde_json::Value;

const TEMPLATES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");


pub struct TemplateData {
    pub template: String,
    pub filename: String,
    pub args: Value,
}

pub struct TemplateService {
    req_sender: Sender<TemplateData>,
    res_receiver: Receiver<String>,
}

impl TemplateService {
    pub fn new(req_sender: Sender<TemplateData>,res_receiver: Receiver<String>)->Self{
        Self{
            req_sender,
            res_receiver,
        }
    }
    pub fn render(&self, template: String, filename: String, args: Value) -> anyhow::Result<String> {
        self.req_sender.send(TemplateData {
            template,
            filename,
            args,
        })?;
        Ok(self.res_receiver.recv()?)
    }

    pub fn render_template(&self, name: &str, args: Value) -> anyhow::Result<String> {
        let template = TEMPLATES_DIR.get_file(name).ok_or(anyhow!("template {} not found.", name))?.contents_utf8().ok_or(anyhow!("utf-8 not supported."))?.to_string();
        self.render(template, name.to_string(), args)
    }
}
