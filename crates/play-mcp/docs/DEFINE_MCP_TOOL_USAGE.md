# define_mcp_tool 宏使用文档

## 目录
- [概述](#概述)
- [前置要求](#前置要求)
- [基本用法](#基本用法)
- [高级用法](#高级用法)
- [工具名称规范](#工具名称规范)
- [参数处理](#参数处理)
- [错误处理](#错误处理)
- [完整示例](#完整示例)
- [常见问题](#常见问题)

## 概述

`define_mcp_tool` 是一个强大的 Rust 宏，用于定义和注册 MCP (Model Context Protocol) 工具。它自动处理工具的元数据加载、参数验证、序列化/反序列化以及工具注册。

### 主要特性
- ✅ 自动工具注册
- ✅ 编译时工具名验证
- ✅ 运行时重复注册检测
- ✅ 自动参数反序列化
- ✅ 支持可选参数 (Option<T>)
- ✅ 支持多种定义模式
- ✅ 自动结构体名称生成

## 前置要求

### 1. 在 `mcp_tools.json` 中定义工具元数据

所有工具必须先在 `src/mcp_tools.json` 文件中定义其元数据：

```json
{
  "tools": [
    {
      "name": "my_tool",
      "description": "我的工具描述",
      "inputSchema": {
        "type": "object",
        "properties": {
          "param1": {
            "type": "string",
            "description": "参数1描述"
          },
          "param2": {
            "type": "number",
            "description": "参数2描述"
          }
        },
        "required": ["param1"],
        "additionalProperties": false
      }
    }
  ]
}
```

### 2. 导入必要的依赖

```rust
use play_mcp::define_mcp_tool;
use serde_json::json;
use anyhow::Result;
```

## 基本用法

### 模式 1: 自动生成结构体名称（推荐用于简单工具）

```rust
define_mcp_tool!(
    "tool_name",
    |param1: String, param2: Option<i32>| {
        // 工具实现逻辑
        Ok(json!({
            "result": format!("处理 {} 和 {:?}", param1, param2)
        }))
    }
);
```

**特点：**
- 无需指定结构体名称，自动生成唯一名称
- 适用于不需要在其他地方引用工具结构体的场景
- 最简洁的语法

### 模式 2: 显式指定结构体名称

```rust
define_mcp_tool!(
    MyToolStruct,
    "tool_name",
    |param1: String, param2: Option<i32>| {
        // 工具实现逻辑
        Ok(json!({
            "result": format!("处理 {} 和 {:?}", param1, param2)
        }))
    }
);
```

**特点：**
- 明确指定结构体名称
- 可以在其他地方引用 `MyToolStruct`
- 适用于需要测试或扩展的工具

### 模式 3: 显式 async move 语法

```rust
define_mcp_tool!(
    MyAsyncTool,
    "async_tool",
    |param: String| async move {
        // 可以使用 await
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        Ok(json!({
            "result": format!("异步处理: {}", param)
        }))
    }
);
```

## 高级用法

### 模式 4: 单参数闭包模式

```rust
define_mcp_tool!(
    SingleParamTool,
    "single_param_tool",
    |input: MyInputStruct| async move {
        // 处理整个输入结构体
        Ok(json!({"result": input.field}))
    }
);
```

### 模式 5: 使用外部函数

```rust
async fn my_tool_handler(input: serde_json::Value) -> Result<serde_json::Value> {
    // 实现逻辑
    Ok(json!({"status": "ok"}))
}

define_mcp_tool!(
    ExternalFuncTool,
    "external_func_tool",
    fn: my_tool_handler
);
```

### 模式 6: 强类型输入输出

```rust
#[derive(Deserialize)]
struct MyInput {
    name: String,
    age: u32,
}

#[derive(Serialize)]
struct MyOutput {
    message: String,
}

async fn process(input: MyInput) -> Result<MyOutput> {
    Ok(MyOutput {
        message: format!("Hello {}, age {}", input.name, input.age)
    })
}

define_mcp_tool!(
    TypedTool,
    "typed_tool",
    input: MyInput,
    output: MyOutput,
    fn: process
);
```

## 工具名称规范

工具名称必须遵循以下规则：

### 允许的字符
- 小写字母: `a-z`
- 数字: `0-9`
- 下划线: `_`
- 冒号: `:`（用于命名空间分隔）
- 点号: `.`（用于版本或子类别）

### 有效的工具名称示例
- ✅ `simple_tool`
- ✅ `tool_v2`
- ✅ `api_2024`
- ✅ `sys:disk`
- ✅ `tool.v1.beta`
- ✅ `namespace:category:tool_123`

### 无效的工具名称示例
- ❌ `MyTool`（包含大写字母）
- ❌ `tool-name`（包含连字符）
- ❌ `tool name`（包含空格）
- ❌ `tool@v1`（包含特殊字符）

## 参数处理

### 必需参数

```rust
define_mcp_tool!(
    "required_params",
    |name: String, age: u32| {
        // name 和 age 都是必需参数
        Ok(json!({
            "greeting": format!("Hello {}, you are {} years old", name, age)
        }))
    }
);
```

### 可选参数

使用 `Option<T>` 定义可选参数：

```rust
define_mcp_tool!(
    "optional_params",
    |name: String, nickname: Option<String>, age: Option<u32>| {
        let display_name = nickname.unwrap_or_else(|| name.clone());
        let age_text = age.map_or("unknown".to_string(), |a| a.to_string());
        
        Ok(json!({
            "message": format!("{} (age: {})", display_name, age_text)
        }))
    }
);
```

### 复杂类型参数

```rust
use std::collections::HashMap;

define_mcp_tool!(
    "complex_params",
    |data: Vec<String>, 
     mapping: HashMap<String, i32>,
     nested: Option<serde_json::Value>| {
        Ok(json!({
            "item_count": data.len(),
            "keys": mapping.keys().collect::<Vec<_>>(),
            "has_nested": nested.is_some()
        }))
    }
);
```

## 错误处理

### 使用 Result 返回类型

```rust
use anyhow::{Result, bail, Context};

define_mcp_tool!(
    "error_handling",
    |input: String| {
        // 输入验证
        if input.is_empty() {
            bail!("输入不能为空");
        }
        
        // 可能失败的操作
        let processed = input.parse::<i32>()
            .context("无法将输入解析为数字")?;
        
        if processed < 0 {
            bail!("数字必须为非负数");
        }
        
        Ok(json!({
            "result": processed * 2
        }))
    }
);
```

### 使用 ? 操作符

```rust
define_mcp_tool!(
    "file_reader",
    |path: String| {
        // 使用 ? 处理可能的错误
        let content = std::fs::read_to_string(&path)?;
        let lines = content.lines().count();
        
        Ok(json!({
            "path": path,
            "lines": lines,
            "size": content.len()
        }))
    }
);
```

## 完整示例

### 示例 1: HTTP 请求工具

```rust
use play_mcp::define_mcp_tool;
use serde_json::json;
use std::collections::HashMap;

define_mcp_tool!(
    "http_request",
    |url: String,
     method: Option<String>,
     headers: Option<HashMap<String, String>>,
     body: Option<String>| {
        
        let method = method.unwrap_or_else(|| "GET".to_string());
        
        // 这里添加实际的 HTTP 请求逻辑
        // 例如使用 reqwest 库
        
        Ok(json!({
            "status": 200,
            "method": method,
            "url": url,
            "headers": headers.unwrap_or_default(),
            "body": body,
            "response": "模拟响应数据"
        }))
    }
);
```

### 示例 2: 数据处理工具

```rust
define_mcp_tool!(
    DataProcessor,
    "process_data",
    |data: Vec<serde_json::Value>, 
     operation: String,
     options: Option<HashMap<String, String>>| {
        
        let result = match operation.as_str() {
            "count" => json!({ "total": data.len() }),
            "filter" => {
                let filter_key = options
                    .as_ref()
                    .and_then(|o| o.get("key"))
                    .ok_or_else(|| anyhow::anyhow!("filter 操作需要 'key' 选项"))?;
                
                let filtered: Vec<_> = data.into_iter()
                    .filter(|item| item.get(filter_key).is_some())
                    .collect();
                
                json!({ "filtered": filtered })
            },
            "transform" => {
                let transformed: Vec<_> = data.into_iter()
                    .map(|mut item| {
                        item["processed"] = json!(true);
                        item
                    })
                    .collect();
                
                json!({ "transformed": transformed })
            },
            _ => bail!("不支持的操作: {}", operation)
        };
        
        Ok(result)
    }
);
```

### 示例 3: 系统信息工具

```rust
define_mcp_tool!(
    "sys_info",
    |component: Option<String>| {
        use sysinfo::{System, SystemExt, CpuExt};
        
        let mut sys = System::new_all();
        sys.refresh_all();
        
        let info = match component.as_deref() {
            Some("cpu") => json!({
                "cores": sys.cpus().len(),
                "brand": sys.cpus().first().map(|c| c.brand()),
                "frequency": sys.cpus().first().map(|c| c.frequency())
            }),
            Some("memory") => json!({
                "total": sys.total_memory(),
                "used": sys.used_memory(),
                "available": sys.available_memory()
            }),
            Some("disk") => {
                let disks: Vec<_> = sys.disks().iter().map(|disk| {
                    json!({
                        "name": disk.name().to_string_lossy(),
                        "mount": disk.mount_point().to_string_lossy(),
                        "total": disk.total_space(),
                        "available": disk.available_space()
                    })
                }).collect();
                json!({ "disks": disks })
            },
            _ => json!({
                "os": sys.name(),
                "kernel": sys.kernel_version(),
                "hostname": sys.host_name(),
                "uptime": sys.uptime()
            })
        };
        
        Ok(info)
    }
);
```

## 常见问题

### Q1: 为什么编译时报错 "Unknown tool name"？

**答**: 工具名称必须先在 `mcp_tools.json` 中定义。检查：
1. 工具名称是否已添加到 `mcp_tools.json`
2. 名称拼写是否一致
3. 是否重新编译（build.rs 需要重新运行）

### Q2: 如何避免重复注册错误？

**答**: 每个工具名称只能注册一次。如果出现重复注册：
1. 检查是否在多个文件中定义了相同的工具
2. 确保没有在循环或多次调用的代码中定义工具
3. 工具定义应该在模块级别，而不是函数内部

### Q3: Option<T> 参数如何处理？

**答**: Option<T> 参数自动处理：
- 如果 JSON 中不包含该字段，值为 `None`
- 如果 JSON 中包含该字段但值为 `null`，值为 `None`
- 否则尝试反序列化为 `Some(T)`

### Q4: 如何调试工具执行？

**答**: 建议添加日志：
```rust
define_mcp_tool!(
    "debug_tool",
    |input: String| {
        tracing::debug!("收到输入: {}", input);
        
        let result = process_input(&input)?;
        tracing::info!("处理结果: {:?}", result);
        
        Ok(json!(result))
    }
);
```

### Q5: 如何测试定义的工具？

**答**: 创建测试用例：
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_my_tool() {
        let tool = MyTool::new();
        let input = json!({
            "param1": "test",
            "param2": 42
        });
        
        let result = tool.execute(input).await.unwrap();
        assert_eq!(result["status"], "success");
    }
}
```

### Q6: 工具名称可以包含哪些字符？

**答**: 允许的字符：
- 小写字母 (a-z)
- 数字 (0-9)
- 下划线 (_)
- 冒号 (:)
- 点号 (.)

### Q7: 如何处理异步操作？

**答**: 所有工具都是异步的，可以直接使用 `.await`：
```rust
define_mcp_tool!(
    "async_tool",
    |url: String| {
        let response = reqwest::get(&url).await?;
        let text = response.text().await?;
        Ok(json!({ "content": text }))
    }
);
```

## 最佳实践

1. **工具命名**: 使用描述性名称，考虑使用命名空间（如 `file:read`, `file:write`）
2. **错误处理**: 总是返回有意义的错误信息
3. **参数验证**: 在处理前验证输入参数
4. **文档**: 在 `mcp_tools.json` 中提供详细的描述和参数说明
5. **测试**: 为每个工具编写单元测试
6. **性能**: 对于耗时操作，考虑添加超时或取消机制

## 总结

`define_mcp_tool` 宏提供了一个强大而灵活的方式来定义 MCP 工具。通过自动化处理序列化、验证和注册，它让开发者可以专注于工具的核心逻辑实现。选择合适的定义模式，遵循命名规范，正确处理错误，你就可以快速创建强大的 MCP 工具。