use play_mcp::metadata_loader;

fn main() {
    println!("Tools from mcp_tools.json (in definition order):\n");
    
    // 获取所有工具名称 - 现在保持定义顺序
    let tool_names = metadata_loader::get_all_tool_names();
    
    for (index, name) in tool_names.iter().enumerate() {
        if let Some(metadata) = metadata_loader::load_tool_metadata(name) {
            println!("{}. {} - {}", 
                index + 1, 
                metadata.name, 
                metadata.description
            );
        }
    }
    
    println!("\nTotal: {} tools", tool_names.len());
    
    // 数组结构的好处：
    println!("\n数组结构的优势：");
    println!("1. 保持工具定义的顺序");
    println!("2. 更容易添加优先级或分组");
    println!("3. 可以轻松实现工具的启用/禁用");
    println!("4. 更符合 JSON 配置的常见模式");
}