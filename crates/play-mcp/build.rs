use std::env;
use std::fs;
use std::path::Path;
use serde_json::Value;

fn main() {
    println!("cargo:rerun-if-changed=src/mcp_tools.json");
    
    // Read mcp_tools.json
    let tools_json_path = Path::new("src/mcp_tools.json");
    let tools_json_content = fs::read_to_string(tools_json_path)
        .expect("Failed to read src/mcp_tools.json");
    
    let tools_config: Value = serde_json::from_str(&tools_json_content)
        .expect("Failed to parse mcp_tools.json");
    
    let tools = tools_config["tools"].as_object()
        .expect("mcp_tools.json should have a 'tools' object");
    
    // Generate Rust code with constants for each tool
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("tool_names.rs");
    
    let mut generated_code = String::new();
    
    // Generate a module with constants
    generated_code.push_str("/// Auto-generated tool name constants from mcp_tools.json\n");
    generated_code.push_str("pub mod tool_names {\n");
    
    for tool_name in tools.keys() {
        let const_name = tool_name.to_uppercase().replace("-", "_");
        generated_code.push_str(&format!(
            "    pub const {}: &str = \"{}\";\n",
            const_name, tool_name
        ));
    }
    
    generated_code.push_str("}\n\n");
    
    // Generate a const function that validates tool names at compile time
    generated_code.push_str("/// Validates tool name at compile time\n");
    generated_code.push_str("pub const fn validate_tool_name(name: &str) -> &str {\n");
    generated_code.push_str("    match name.as_bytes() {\n");
    
    for tool_name in tools.keys() {
        generated_code.push_str(&format!(
            "        b\"{}\" => \"{}\",\n",
            tool_name, tool_name
        ));
    }
    
    let valid_names: Vec<String> = tools.keys().cloned().collect();
    generated_code.push_str(&format!(
        "        _ => panic!(\"Unknown tool name. Tool name must match one defined in mcp_tools.json. Valid names are: {}\"),\n",
        valid_names.join(", ")
    ));
    generated_code.push_str("    }\n");
    generated_code.push_str("}\n");
    
    fs::write(&dest_path, generated_code)
        .expect("Failed to write generated code");
    
    println!("cargo:rustc-env=TOOL_NAMES_RS={}", dest_path.display());
}