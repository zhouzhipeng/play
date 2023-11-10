use rustpython_vm;

use crate::service::template_service::{TemplateData, TemplateService};
use crate::tables::DBPool;

pub mod controller;
pub mod threads;
pub mod tables;
pub mod service;


pub struct AppState {
    pub template_service: TemplateService,
    pub db: DBPool,
}