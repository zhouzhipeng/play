use crossbeam_channel::{Receiver, Sender};
use include_dir::{Dir, include_dir};
use rustpython_vm;

use crate::service::template_service::TemplateData;
use crate::tables::DBPool;

pub static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

///
/// templates files


pub struct AppState {
    pub req_sender: Sender<TemplateData>,
    pub res_receiver: Receiver<String>,
    pub db: DBPool,
}



pub mod controller;
pub mod threads;
pub mod tables;
pub mod service;