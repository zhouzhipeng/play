use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct EchoInput {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct EchoOutput {
    pub echoed: String,
    pub timestamp: String,
}

async fn execute_echo(input: EchoInput) -> Result<EchoOutput> {
    Ok(EchoOutput {
        echoed: input.message,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

// Define the entire tool with one macro call
crate::define_mcp_tool!(
    EchoTool,
    "echo",
    input: EchoInput,
    output: EchoOutput,
    fn: execute_echo
);