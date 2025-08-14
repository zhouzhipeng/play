use play_mcp::tools::{Tool, BilibiliDownloadTool};
use serde_json::json;

#[tokio::main]
async fn main() {
    let tool = BilibiliDownloadTool::new();
    
    let metadata = tool.metadata();
    println!("Tool Name: {}", metadata.name);
    println!("Description: {}", metadata.description);
    println!("Input Schema: {}", serde_json::to_string_pretty(&metadata.input_schema).unwrap());
    
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