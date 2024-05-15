use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageList {
    pub object: String,
    pub data: Vec<Message>,
    pub first_id: String,
    pub last_id: String,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub thread_id: String,
    pub role: String,
    pub content: Vec<Content>,
    pub assistant_id: Option<serde_json::Value>,
    pub run_id: Option<serde_json::Value>,
    pub metadata: Metadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Text,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Text {
    pub value: String,
    pub annotations: Vec<Option<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
}
