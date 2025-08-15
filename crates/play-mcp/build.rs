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
    
    let tools = tools_config["tools"].as_array()
        .expect("mcp_tools.json should have a 'tools' array");
    
    // Generate Rust code with constants for each tool
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("tool_names.rs");
    
    let mut generated_code = String::new();
    
    // Collect tool names and generate constants
    let mut const_definitions = Vec::new();
    let mut match_arms = Vec::new();
    let mut tool_names = Vec::new();
    let mut uniqueness_arms = Vec::new();
    
    for tool in tools {
        let tool_name = tool["name"].as_str()
            .expect("Each tool must have a 'name' field");
        
        // Validate tool name format
        if !tool_name.chars().all(|c| c.is_ascii_lowercase() || c == '_' || c == ':' || c == '.') {
            panic!(
                "Invalid tool name '{}'. Tool names must only contain lowercase letters, underscore (_), colon (:), or dot (.)",
                tool_name
            );
        }
        
        // Generate const name by replacing special chars
        let const_name = tool_name.to_uppercase()
            .replace("-", "_")
            .replace(":", "_")
            .replace(".", "_");
        
        const_definitions.push(format!(
            "    pub const {}: &str = \"{}\";",
            const_name, tool_name
        ));
        
        match_arms.push(format!(
            "        b\"{}\" => \"{}\",",
            tool_name, tool_name
        ));
        
        tool_names.push(tool_name.to_string());
        
        // Generate uniqueness check arm
        uniqueness_arms.push(format!(
            "    (\"{}\") => {{\n        \
             #[doc(hidden)]\n        \
             const __TOOL_REGISTRATION_GUARD_{}: () = ();\n    \
             }};",
            tool_name, const_name
        ));
    }
    
    // Generate the complete code
    generated_code.push_str(&format!(
        "/// Auto-generated tool name constants from mcp_tools.json\n\
         pub mod tool_names {{\n\
         {}\n\
         }}\n\n\
         /// Validates tool name at compile time\n\
         pub const fn validate_tool_name(name: &str) -> &str {{\n\
             match name.as_bytes() {{\n\
         {}\n\
                 _ => panic!(\"Unknown tool name. Tool name must match one defined in mcp_tools.json. Valid names are: {}\"),\n\
             }}\n\
         }}\n\n\
         /// Internal macro to ensure tool uniqueness at compile time\n\
         /// This creates a const item that will cause a compile error if duplicated\n\
         #[macro_export]\n\
         #[doc(hidden)]\n\
         macro_rules! __internal_ensure_unique_tool {{\n\
         {}\n\
             ($other:expr) => {{\n\
                 compile_error!(concat!(\"Unknown tool name: \", $other, \". Tool must be defined in mcp_tools.json\"));\n\
             }};\n\
         }}",
        const_definitions.join("\n"),
        match_arms.join("\n"),
        tool_names.join(", "),
        uniqueness_arms.join("\n")
    ));
    
    fs::write(&dest_path, generated_code)
        .expect("Failed to write generated code");
    
    println!("cargo:rustc-env=TOOL_NAMES_RS={}", dest_path.display());
}