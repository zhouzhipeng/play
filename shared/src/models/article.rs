use serde::{Deserialize, Serialize};


pub const ADD_ARTICLE: &str = "/article/add";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArticleVo {
    pub id: i64,
    pub title: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
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

