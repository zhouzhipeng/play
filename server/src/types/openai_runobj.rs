use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RunObj {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub assistant_id: String,
    pub thread_id: String,
    pub status: String,
    pub started_at: i64,
    pub expires_at: i64,
    pub cancelled_at: Option<serde_json::Value>,
    pub failed_at: Option<serde_json::Value>,
    pub completed_at: Option<serde_json::Value>,
    pub required_action: RequiredAction,
    pub last_error: Option<serde_json::Value>,
    pub model: String,
    pub instructions: String,
    pub file_ids: Vec<Option<serde_json::Value>>,
    pub metadata: Metadata,
    pub temperature: f64,
    pub usage: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequiredAction {
    #[serde(rename = "type")]
    pub required_action_type: String,
    pub submit_tool_outputs: SubmitToolOutputs,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitToolOutputs {
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_call_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Parameters,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Parameters {
    #[serde(rename = "type")]
    pub parameters_type: String,
    pub properties: Properties,
    pub required: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Properties {
    pub location: Location,
    pub unit: Unit,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    #[serde(rename = "type")]
    pub location_type: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Unit {
    #[serde(rename = "type")]
    pub unit_type: String,
    #[serde(rename = "enum")]
    pub unit_enum: Vec<String>,
}
