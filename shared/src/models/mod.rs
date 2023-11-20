use anyhow::{anyhow, bail};
use reqwest::{Client, Response, Url};
use crate::constants::HOST;

pub mod user;
pub mod article;
#[cfg(feature = "wasm-bindgen")]
mod wasm_traits_impl;

pub fn check_response(response: &Response) -> anyhow::Result<()> {
    if !response.status().is_success() {
        bail!("request error! >> status : {}", response.status())
    }
    Ok(())
}

pub struct RequestClient{
    pub host : String,
    pub client : Client,
}

impl Default for RequestClient{
    fn default() -> Self {
        RequestClient{
            host: HOST.to_string(),
            client: Client::new(),
        }
    }
}

impl RequestClient{
    fn url(&self, url: &str)->anyhow::Result<Url>{
        Ok(Url::parse(self.host.as_str())?.join(url)?)
    }
}