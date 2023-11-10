use crossbeam_channel::{Receiver, Sender};
use include_dir::{Dir, include_dir};
use rustpython_vm;
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
    pub fn render(&self, template: String, filename: String, args: Value) -> String {
        self.req_sender.send(TemplateData {
            template,
            filename,
            args,
        }).expect("send error");
        self.res_receiver.recv().unwrap()
    }

    pub fn render_template(&self, name: &str, args: Value) -> String {
        let template = TEMPLATES_DIR.get_file(name).unwrap().contents_utf8().unwrap().to_string();
        self.render(template, name.to_string(), args)
    }
}
