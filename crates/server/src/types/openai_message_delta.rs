use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageDelta {
    pub id: String,
    pub object: String,
    pub delta: Delta,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Delta {
    pub content: Vec<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    pub index: i64,
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Text,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Text {
    pub value: String,
}
