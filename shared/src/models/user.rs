use reqwest::Client;
use serde::{Deserialize, Serialize};

#[cfg(ENV = "dev")]
pub const HOST: &str = "http://localhost:3000";
#[cfg(ENV = "prod")]
pub const HOST: &str = "http://127.0.0.1:3000";

pub const USER_LIST: &str = "/users";
pub const ADD_USER: &str = "/add-user";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserVo {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct AddUser {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateUser {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct QueryUser {
    pub name: String,
}

pub async fn add_user(user: AddUser) -> anyhow::Result<String> {
    let body = Client::new().get(format!("{}{}",HOST, ADD_USER)).query(&user).send().await?.text().await?;
    Ok(body)
}

pub async fn query_users(user: QueryUser) -> anyhow::Result<Vec<UserVo>> {
    let body = Client::new().get(format!("{}{}",HOST, USER_LIST)).query(&user).send().await?.json::<Vec<UserVo>>().await?;
    Ok(body)
}

