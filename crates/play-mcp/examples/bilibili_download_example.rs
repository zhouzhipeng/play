use play_mcp::tools::{Tool, BilibiliDownloadTool};
use serde_json::json;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();

    let tool = BilibiliDownloadTool::new();
    
    println!("Tool Name: {}", tool.name());
    println!("Description: {}", tool.description());
    println!("Input Schema: {}", serde_json::to_string_pretty(&tool.input_schema()).unwrap());
    
    let input = json!({
        "url": "https://www.bilibili.com/video/BV14rt1zFECj/?spm_id_from=333.337.search-card.all.click",
        "quality": "720p"
    });
    
    println!("\nExecuting download with input: {}", serde_json::to_string_pretty(&input).unwrap());
    
    match tool.execute(input).await {
        Ok(result) => {
            println!("Result: {}", serde_json::to_string_pretty(&result).unwrap());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    tokio::time::sleep(std::time::Duration::from_secs(10000)).await;
}