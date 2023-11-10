use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use rustpython_vm;
use serde_json::Value;


use include_dir::{include_dir, Dir};

pub static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

///
/// static files
pub const TEST_HTML: &'static str = include_str!("../templates/test.html");

pub struct TemplateData {
    pub template: String,
    pub args: Value,
}

pub struct AppState {
    pub req_sender: Sender<TemplateData>,
    pub res_receiver: Receiver<String>,
}


pub fn render(state: Arc<AppState>, template: String, args: Value) -> String {
    state.req_sender.send(TemplateData {
        template,
        args,
    }).expect("send error");
    state.res_receiver.recv().unwrap()
}


pub mod controller;
pub mod threads;