use serde::{Deserialize, Serialize};



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



