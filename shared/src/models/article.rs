use anyhow::{anyhow, bail};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};

use crate::constants::API_ARTICLE_ADD;
use crate::models::{check_response, RequestClient};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArticleVo {
    pub id: i64,
    pub title: String,
    pub content: String,
}

#[derive(Serialize, Deserialize,Debug)]
pub struct AddArticle {
    pub title: String,
    pub content: String,
}


#[derive(Serialize, Deserialize)]
pub struct UpdateArticle {
    pub title: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct QueryArticle {
    pub title: String,

}

impl RequestClient{
    pub async fn api_article_add(&self, add_article: &AddArticle) -> anyhow::Result<String> {
        let response = self.client.post(self.url(API_ARTICLE_ADD)?).form(add_article).send().await?;
        check_response(&response)?;
        let body = response.text().await?;
        Ok(body)
    }
}
