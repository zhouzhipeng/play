use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::tools::ToolMetadata;

// Include the generated validation code
include!(concat!(env!("OUT_DIR"), "/tool_names.rs"));

// Re-export the macro so it's available to users
pub use __internal_ensure_unique_tool;

/// Raw tool definition from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Root structure of mcp_tools.json
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolsConfig {
    tools: Vec<ToolDefinition>,
}

/// Static tools configuration loaded from mcp_tools.json
static TOOLS_CONFIG: Lazy<ToolsConfig> = Lazy::new(|| {
    let json_str = include_str!("mcp_tools.json");
    serde_json::from_str(json_str).expect("Failed to parse mcp_tools.json")
});

/// Static HashMap for quick tool lookup by name
static TOOLS_MAP: Lazy<HashMap<String, ToolDefinition>> = Lazy::new(|| {
    TOOLS_CONFIG.tools
        .iter()
        .map(|tool| (tool.name.clone(), tool.clone()))
        .collect()
});

/// Load metadata for a specific tool
pub fn load_tool_metadata(tool_name: &str) -> Option<ToolMetadata> {
    TOOLS_MAP.get(tool_name).map(|def| {
        ToolMetadata::new(
            def.name.clone(),
            def.description.clone(),
            def.input_schema.clone(),
        )
    })
}

/// Load a tool definition
pub fn load_tool_definition(tool_name: &str) -> Option<ToolDefinition> {
    TOOLS_MAP.get(tool_name).cloned()
}

/// Validate tool name characters at compile time
#[doc(hidden)]
pub const fn validate_tool_name_chars(name: &str) -> bool {
    let bytes = name.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        // Allow: a-z, _, :, .
        if !((c >= b'a' && c <= b'z') || c == b'_' || c == b':' || c == b'.') {
            return false;
        }
        i += 1;
    }
    true
}


/// Define a complete MCP tool with struct, impl, and registration
#[macro_export]
macro_rules! define_mcp_tool {
    // Tool without struct name - auto-generate unique struct  
    (
        $tool_key:expr,
        |$($param:ident : $param_type:ty),+| $body:block
    ) => {
        
        // Use a const _ block to encapsulate the auto-generated tool
        // This creates a unique context for each macro invocation
        const _: () = {
            use $crate::tools::{Tool, ToolMetadata, ToolFactory, TOOL_FACTORIES};
            use $crate::metadata_loader::{validate_tool_name, validate_tool_name_chars, load_tool_metadata};
            
            struct AutoGenTool {
                metadata: ToolMetadata,
            }
            
            impl AutoGenTool {
                pub fn new() -> Self {
                    // Validate tool name format and existence at compile time
                    const _: () = {
                        if !validate_tool_name_chars($tool_key) {
                            panic!("Invalid tool name format. Tool names must only contain lowercase letters, underscore (_), colon (:), or dot (.)");
                        }
                    };
                    const TOOL_NAME: &str = validate_tool_name($tool_key);
                    
                    let metadata = load_tool_metadata(TOOL_NAME)
                        .expect("Failed to load metadata for tool");
                    Self { metadata }
                }
            }
            
            impl Default for AutoGenTool {
                fn default() -> Self {
                    Self::new()
                }
            }
            
            #[async_trait::async_trait]
            impl Tool for AutoGenTool {
                fn metadata(&self) -> &ToolMetadata {
                    &self.metadata
                }
                
                async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
                    // Extract individual parameters from JSON
                    $( 
                        let $param: $param_type = {
                            let field_value = input.get(stringify!($param));
                            
                            if field_value.is_none() {
                                match serde_json::from_value::<$param_type>(serde_json::Value::Null) {
                                    Ok(val) => val,
                                    Err(_) => {
                                        return Err(anyhow::anyhow!("Missing required field: {}", stringify!($param)));
                                    }
                                }
                            } else {
                                serde_json::from_value(field_value.unwrap().clone())?
                            }
                        };
                    )+
                    
                    let execute_fn = |$($param: $param_type),+| async move $body;
                    execute_fn($($param),+).await
                }
            }
            
            // Auto-register the tool factory 
            #[linkme::distributed_slice(TOOL_FACTORIES)]
            static AUTO_GEN_REGISTER: ToolFactory = || {
                Box::new(AutoGenTool::new())
            };
            
            ()
        };
    };
    
    // Simple tool with multiple closure parameters (implicit async)
    (
        $struct_name:ident,
        $tool_key:expr,
        |$($param:ident : $param_type:ty),+| $body:block
    ) => {
        pub struct $struct_name {
            metadata: $crate::tools::ToolMetadata,
        }
        
        impl $struct_name {
            pub fn new() -> Self {
                // Validate tool name format and existence at compile time
                const _: () = {
                    if !$crate::metadata_loader::validate_tool_name_chars($tool_key) {
                        panic!("Invalid tool name format. Tool names must only contain lowercase letters, underscore (_), colon (:), or dot (.)");
                    }
                };
                const TOOL_NAME: &str = $crate::metadata_loader::validate_tool_name($tool_key);
                
                let metadata = $crate::metadata_loader::load_tool_metadata(TOOL_NAME)
                    .expect(concat!("Failed to load metadata for ", $tool_key));
                Self { metadata }
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new()
            }
        }
        
        // Auto-register the tool factory in an anonymous const to avoid name conflicts
        const _: () = {
            #[linkme::distributed_slice($crate::tools::TOOL_FACTORIES)]
            #[allow(non_upper_case_globals)]
            static __ToolReg: $crate::tools::ToolFactory = || {
                Box::new($struct_name::new())
            };
        };
        
        #[async_trait::async_trait]
        impl $crate::tools::Tool for $struct_name {
            fn metadata(&self) -> &$crate::tools::ToolMetadata {
                &self.metadata
            }
            
            async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
                // Extract individual parameters from JSON
                $(
                    let $param: $param_type = {
                        // Check if this is an Option type by trying to deserialize null
                        let field_value = input.get(stringify!($param));
                        
                        if field_value.is_none() {
                            // If field doesn't exist, try to deserialize null (works for Option<T>)
                            match serde_json::from_value::<$param_type>(serde_json::Value::Null) {
                                Ok(val) => val,
                                Err(_) => {
                                    // Not an Option type, field is required
                                    return Err(anyhow::anyhow!("Missing required field: {}", stringify!($param)));
                                }
                            }
                        } else {
                            // Field exists, deserialize it
                            serde_json::from_value(field_value.unwrap().clone())?
                        }
                    };
                )+
                
                let execute_fn = |$($param: $param_type),+| async move $body;
                execute_fn($($param),+).await
            }
        }
    };
    
    // Simple tool with multiple closure parameters (explicit async move)
    (
        $struct_name:ident,
        $tool_key:expr,
        |$($param:ident : $param_type:ty),+| async move $body:block
    ) => {
        pub struct $struct_name {
            metadata: $crate::tools::ToolMetadata,
        }
        
        impl $struct_name {
            pub fn new() -> Self {
                // Validate tool name format and existence at compile time
                const _: () = {
                    if !$crate::metadata_loader::validate_tool_name_chars($tool_key) {
                        panic!("Invalid tool name format. Tool names must only contain lowercase letters, underscore (_), colon (:), or dot (.)");
                    }
                };
                const TOOL_NAME: &str = $crate::metadata_loader::validate_tool_name($tool_key);
                
                let metadata = $crate::metadata_loader::load_tool_metadata(TOOL_NAME)
                    .expect(concat!("Failed to load metadata for ", $tool_key));
                Self { metadata }
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new()
            }
        }
        
        // Auto-register the tool factory - unique name per invocation
        #[linkme::distributed_slice($crate::tools::TOOL_FACTORIES)]
        #[allow(non_upper_case_globals)]
        static __ToolReg: $crate::tools::ToolFactory = || {
            Box::new($struct_name::new())
        };
        
        #[async_trait::async_trait]
        impl $crate::tools::Tool for $struct_name {
            fn metadata(&self) -> &$crate::tools::ToolMetadata {
                &self.metadata
            }
            
            async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
                // Extract individual parameters from JSON
                $(
                    let $param: $param_type = {
                        // Check if this is an Option type by trying to deserialize null
                        let field_value = input.get(stringify!($param));
                        
                        if field_value.is_none() {
                            // If field doesn't exist, try to deserialize null (works for Option<T>)
                            match serde_json::from_value::<$param_type>(serde_json::Value::Null) {
                                Ok(val) => val,
                                Err(_) => {
                                    // Not an Option type, field is required
                                    return Err(anyhow::anyhow!("Missing required field: {}", stringify!($param)));
                                }
                            }
                        } else {
                            // Field exists, deserialize it
                            serde_json::from_value(field_value.unwrap().clone())?
                        }
                    };
                )+
                
                let execute_fn = |$($param: $param_type),+| $body;
                execute_fn($($param),+).await
            }
        }
    };
    
    // Simple tool without extra fields - closure version (single parameter)
    (
        $struct_name:ident,
        $tool_key:expr,
        $execute_body:expr
    ) => {
        pub struct $struct_name {
            metadata: $crate::tools::ToolMetadata,
        }
        
        impl $struct_name {
            pub fn new() -> Self {
                // Validate tool name format and existence at compile time
                const _: () = {
                    if !$crate::metadata_loader::validate_tool_name_chars($tool_key) {
                        panic!("Invalid tool name format. Tool names must only contain lowercase letters, underscore (_), colon (:), or dot (.)");
                    }
                };
                const TOOL_NAME: &str = $crate::metadata_loader::validate_tool_name($tool_key);
                
                let metadata = $crate::metadata_loader::load_tool_metadata(TOOL_NAME)
                    .expect(concat!("Failed to load metadata for ", $tool_key));
                Self { metadata }
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new()
            }
        }
        
        // Auto-register the tool factory - unique name per invocation
        #[linkme::distributed_slice($crate::tools::TOOL_FACTORIES)]
        #[allow(non_upper_case_globals)]
        static __ToolReg: $crate::tools::ToolFactory = || {
            Box::new($struct_name::new())
        };
        
        #[async_trait::async_trait]
        impl $crate::tools::Tool for $struct_name {
            fn metadata(&self) -> &$crate::tools::ToolMetadata {
                &self.metadata
            }
            
            async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
                let execute_fn = $execute_body;
                execute_fn(serde_json::from_value(input)?).await
            }
        }
    };
    
    // Simple tool without extra fields - function name version
    (
        $struct_name:ident,
        $tool_key:expr,
        fn: $execute_fn:path
    ) => {
        pub struct $struct_name {
            metadata: $crate::tools::ToolMetadata,
        }
        
        impl $struct_name {
            pub fn new() -> Self {
                // Validate tool name format and existence at compile time
                const _: () = {
                    if !$crate::metadata_loader::validate_tool_name_chars($tool_key) {
                        panic!("Invalid tool name format. Tool names must only contain lowercase letters, underscore (_), colon (:), or dot (.)");
                    }
                };
                const TOOL_NAME: &str = $crate::metadata_loader::validate_tool_name($tool_key);
                
                let metadata = $crate::metadata_loader::load_tool_metadata(TOOL_NAME)
                    .expect(concat!("Failed to load metadata for ", $tool_key));
                Self { metadata }
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new()
            }
        }
        
        // Auto-register the tool factory - unique name per invocation
        #[linkme::distributed_slice($crate::tools::TOOL_FACTORIES)]
        #[allow(non_upper_case_globals)]
        static __ToolReg: $crate::tools::ToolFactory = || {
            Box::new($struct_name::new())
        };
        
        #[async_trait::async_trait]
        impl $crate::tools::Tool for $struct_name {
            fn metadata(&self) -> &$crate::tools::ToolMetadata {
                &self.metadata
            }
            
            async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
                $execute_fn(input).await
            }
        }
    };
    
    // Simple tool with generic input/output types - function name version
    (
        $struct_name:ident,
        $tool_key:expr,
        input: $input_type:ty,
        output: $output_type:ty,
        fn: $execute_fn:path
    ) => {
        pub struct $struct_name {
            metadata: $crate::tools::ToolMetadata,
        }
        
        impl $struct_name {
            pub fn new() -> Self {
                // Validate tool name format and existence at compile time
                const _: () = {
                    if !$crate::metadata_loader::validate_tool_name_chars($tool_key) {
                        panic!("Invalid tool name format. Tool names must only contain lowercase letters, underscore (_), colon (:), or dot (.)");
                    }
                };
                const TOOL_NAME: &str = $crate::metadata_loader::validate_tool_name($tool_key);
                
                let metadata = $crate::metadata_loader::load_tool_metadata(TOOL_NAME)
                    .expect(concat!("Failed to load metadata for ", $tool_key));
                Self { metadata }
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new()
            }
        }
        
        // Auto-register the tool factory - unique name per invocation
        #[linkme::distributed_slice($crate::tools::TOOL_FACTORIES)]
        #[allow(non_upper_case_globals)]
        static __ToolReg: $crate::tools::ToolFactory = || {
            Box::new($struct_name::new())
        };
        
        #[async_trait::async_trait]
        impl $crate::tools::Tool for $struct_name {
            fn metadata(&self) -> &$crate::tools::ToolMetadata {
                &self.metadata
            }
            
            async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
                let typed_input: $input_type = serde_json::from_value(input)?;
                let result: $output_type = $execute_fn(typed_input).await?;
                Ok(serde_json::to_value(result)?)
            }
        }
    };
    
    // Tool with custom fields - closure version
    (
        $struct_name:ident,
        $tool_key:expr,
        fields: { $($field:ident : $field_type:ty = $field_init:expr),* },
        $execute_body:expr
    ) => {
        pub struct $struct_name {
            metadata: $crate::tools::ToolMetadata,
            $(pub $field: $field_type),*
        }
        
        impl $struct_name {
            pub fn new() -> Self {
                const TOOL_NAME: &str = $crate::metadata_loader::validate_tool_name($tool_key);
                
                let metadata = $crate::metadata_loader::load_tool_metadata(TOOL_NAME)
                    .expect(concat!("Failed to load metadata for ", $tool_key));
                Self { 
                    metadata,
                    $($field: $field_init),*
                }
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new()
            }
        }
        
        #[linkme::distributed_slice($crate::tools::TOOL_FACTORIES)]
        #[allow(non_upper_case_globals)]
        static __ToolReg: $crate::tools::ToolFactory = || {
            Box::new($struct_name::new())
        };
        
        #[async_trait::async_trait]
        impl $crate::tools::Tool for $struct_name {
            fn metadata(&self) -> &$crate::tools::ToolMetadata {
                &self.metadata
            }
            
            async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
                let execute_fn = $execute_body;
                execute_fn(self, input).await
            }
        }
    };
    
    // Tool with custom fields - function name version
    (
        $struct_name:ident,
        $tool_key:expr,
        fields: { $($field:ident : $field_type:ty = $field_init:expr),* },
        fn: $execute_fn:path
    ) => {
        pub struct $struct_name {
            metadata: $crate::tools::ToolMetadata,
            $(pub $field: $field_type),*
        }
        
        impl $struct_name {
            pub fn new() -> Self {
                const TOOL_NAME: &str = $crate::metadata_loader::validate_tool_name($tool_key);
                
                let metadata = $crate::metadata_loader::load_tool_metadata(TOOL_NAME)
                    .expect(concat!("Failed to load metadata for ", $tool_key));
                Self { 
                    metadata,
                    $($field: $field_init),*
                }
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new()
            }
        }
        
        #[linkme::distributed_slice($crate::tools::TOOL_FACTORIES)]
        #[allow(non_upper_case_globals)]
        static __ToolReg: $crate::tools::ToolFactory = || {
            Box::new($struct_name::new())
        };
        
        #[async_trait::async_trait]
        impl $crate::tools::Tool for $struct_name {
            fn metadata(&self) -> &$crate::tools::ToolMetadata {
                &self.metadata
            }
            
            async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
                $execute_fn(self, input).await
            }
        }
    };
    
    // Tool with custom fields and generic input/output types - function name version
    (
        $struct_name:ident,
        $tool_key:expr,
        fields: { $($field:ident : $field_type:ty = $field_init:expr),* },
        input: $input_type:ty,
        output: $output_type:ty,
        fn: $execute_fn:path
    ) => {
        pub struct $struct_name {
            metadata: $crate::tools::ToolMetadata,
            $(pub $field: $field_type),*
        }
        
        impl $struct_name {
            pub fn new() -> Self {
                const TOOL_NAME: &str = $crate::metadata_loader::validate_tool_name($tool_key);
                
                let metadata = $crate::metadata_loader::load_tool_metadata(TOOL_NAME)
                    .expect(concat!("Failed to load metadata for ", $tool_key));
                Self { 
                    metadata,
                    $($field: $field_init),*
                }
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new()
            }
        }
        
        #[linkme::distributed_slice($crate::tools::TOOL_FACTORIES)]
        #[allow(non_upper_case_globals)]
        static __ToolReg: $crate::tools::ToolFactory = || {
            Box::new($struct_name::new())
        };
        
        #[async_trait::async_trait]
        impl $crate::tools::Tool for $struct_name {
            fn metadata(&self) -> &$crate::tools::ToolMetadata {
                &self.metadata
            }
            
            async fn execute(&self, input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
                let typed_input: $input_type = serde_json::from_value(input)?;
                let result: $output_type = $execute_fn(self, typed_input).await?;
                Ok(serde_json::to_value(result)?)
            }
        }
    };
}

/// Get all available tool names
pub fn get_all_tool_names() -> Vec<String> {
    TOOLS_CONFIG.tools
        .iter()
        .map(|tool| tool.name.clone())
        .collect()
}

/// Check if a tool exists
pub fn tool_exists(tool_name: &str) -> bool {
    TOOLS_MAP.contains_key(tool_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_tool_metadata() {
        let metadata = load_tool_metadata("echo");
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        assert_eq!(metadata.name, "echo");
    }

    #[test]
    fn test_get_all_tool_names() {
        let names = get_all_tool_names();
        assert!(names.contains(&"echo".to_string()));
        assert!(names.contains(&"sys_info".to_string()));
    }
}