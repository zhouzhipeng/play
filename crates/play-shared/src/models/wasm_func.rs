use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize,Debug)]
pub struct Request {
    pub headers: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub body: String,
    pub url: String,
}

#[derive(Serialize,Deserialize,Debug)]
pub struct Response {
    pub headers: HashMap<String, String>,
    pub body: String,
    pub status_code: u16,
}