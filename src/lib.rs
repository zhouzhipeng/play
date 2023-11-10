use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use rustpython_vm;
use serde_json::Value;


use include_dir::{include_dir, Dir};

pub static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

///
/// templates files
pub const TEMPLATES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

pub struct TemplateData {
    pub template: String,
    pub filename: String,
    pub args: Value,
}

pub struct AppState {
    pub req_sender: Sender<TemplateData>,
    pub res_receiver: Receiver<String>,
}


pub fn render(state: Arc<AppState>, template: String, filename: String, args: Value) -> String {
    state.req_sender.send(TemplateData {
        template,
        filename,
        args,
    }).expect("send error");
    state.res_receiver.recv().unwrap()
}

pub fn render_template(state: Arc<AppState>, name: &str, args: Value) -> String {
    let template = TEMPLATES_DIR.get_file(name).unwrap().contents_utf8().unwrap().to_string();
    render(state, template, name.to_string(), args)
}



pub mod controller;
pub mod threads;