# play-dylib 动态库插件系统使用指南

## 概述

play-dylib 是一个强大的动态库插件系统，允许在运行时加载和执行外部代码。该系统由三个核心 crate 组成：

- **play-dylib-abi**: 定义应用程序二进制接口（ABI）和数据结构
- **play-dylib-loader**: 提供动态库的运行时加载器
- **play-dylib-example**: 演示如何创建插件的示例代码

## 系统架构

### 1. ABI 定义层 (play-dylib-abi)

该 crate 定义了宿主应用和插件之间的契约，确保二进制兼容性。

#### HTTP 请求处理

```rust
// HTTP 请求结构
pub struct HttpRequest {
    pub method: String,           // HTTP 方法
    pub headers: HashMap<String, String>,  // 请求头
    pub query: HashMap<String, String>,    // 查询参数
    pub url: String,              // 请求 URL
    pub body: Option<String>,     // 请求体
    pub host_context: HostContext, // 宿主上下文
}

// HTTP 响应结构
pub struct HttpResponse {
    pub headers: HashMap<String, String>,  // 响应头
    pub body: String,             // 响应体
    pub status_code: u16,         // 状态码
    pub error: Option<String>,    // 错误信息
}
```

#### 宿主上下文

```rust
pub struct HostContext {
    pub host_url: String,         // 宿主 URL
    pub plugin_prefix: String,    // 插件前缀路径
    pub data_dir: String,         // 数据目录
    pub config: Option<String>,   // 配置信息
}
```

### 2. 加载器层 (play-dylib-loader)

负责在运行时动态加载和执行插件库。

主要功能：
- 动态加载 `.so`/`.dylib` 文件
- 支持热重载（可选功能）
- 插件缓存管理
- 全面的错误处理和 panic 捕获
- 完整的异步支持

### 3. 插件实现层

插件开发者使用 ABI 定义来创建兼容的动态库。

## 快速开始

### 1. 创建一个简单的插件

创建新的 Rust 项目：

```bash
cargo new --lib my-plugin
cd my-plugin
```

编辑 `Cargo.toml`：

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # 重要：必须编译为 C 动态库

[dependencies]
play-dylib-abi = { path = "../play-dylib-abi" }
anyhow = "1.0"
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
```

### 2. 实现插件处理器

#### 同步处理器示例

```rust
use play_dylib_abi::*;
use anyhow::Result;

fn handle_request(req: HttpRequest) -> Result<HttpResponse> {
    // 解析查询参数
    let name = req.query.get("name").unwrap_or(&"World".to_string());
    
    // 返回响应
    Ok(HttpResponse::text(format!("Hello, {}!", name)))
}

// 使用宏生成必要的导出函数
request_handler!(handle_request);
```

#### 异步处理器示例

```rust
use play_dylib_abi::*;
use anyhow::Result;

async fn handle_async(req: HttpRequest) -> Result<HttpResponse> {
    // 解析 JSON 请求体
    if let Some(body) = req.body {
        let data: serde_json::Value = serde_json::from_str(&body)?;
        
        // 异步处理逻辑
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // 返回 JSON 响应
        Ok(HttpResponse::json(&serde_json::json!({
            "message": "处理成功",
            "input": data
        })))
    } else {
        Ok(HttpResponse::text("请提供请求体"))
    }
}

// 使用异步宏
async_request_handler!(handle_async);
```

### 3. 实现长时间运行的服务器插件

```rust
use play_dylib_abi::*;

async fn run_server(ctx: HostContext) {
    println!("服务器插件启动");
    println!("数据目录: {}", ctx.data_dir);
    
    // 读取配置
    if let Some(config) = ctx.config {
        println!("配置: {}", config);
    }
    
    // 启动服务器循环
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        println!("服务器运行中...");
    }
}

// 使用服务器宏
async_run!(run_server);
```

### 4. 编译插件

```bash
# 编译为动态库
cargo build --release

# 输出文件位置
# Linux: target/release/libmy_plugin.so
# macOS: target/release/libmy_plugin.dylib
# Windows: target/release/my_plugin.dll
```

## 在宿主应用中使用插件

### 1. 添加加载器依赖

```toml
[dependencies]
play-dylib-loader = { path = "../play-dylib-loader" }
play-dylib-abi = { path = "../play-dylib-abi" }
```

### 2. 加载并调用插件

```rust
use play_dylib_loader::load_and_run;
use play_dylib_abi::{HttpRequest, HostContext};

async fn handle_plugin_request() {
    // 构建请求
    let request = HttpRequest {
        method: "GET".to_string(),
        headers: HashMap::new(),
        query: HashMap::from([("name".to_string(), "张三".to_string())]),
        url: "/api/hello".to_string(),
        body: None,
        host_context: HostContext {
            host_url: "http://localhost:8080".to_string(),
            plugin_prefix: "/plugins/my-plugin".to_string(),
            data_dir: "/var/data/my-plugin".to_string(),
            config: Some(r#"{"debug": true}"#.to_string()),
        },
    };
    
    // 加载并执行插件
    match load_and_run("./plugins/libmy_plugin.so", request).await {
        Ok(response) => {
            println!("状态码: {}", response.status_code);
            println!("响应: {}", response.body);
        }
        Err(e) => {
            eprintln!("插件执行失败: {}", e);
        }
    }
}
```

### 3. 加载服务器插件

```rust
use play_dylib_loader::load_and_run_server;

async fn start_plugin_server() {
    let host_context = HostContext {
        host_url: "http://localhost:8080".to_string(),
        plugin_prefix: "/plugins/server".to_string(),
        data_dir: "/var/data/server-plugin".to_string(),
        config: None,
    };
    
    // 启动服务器插件（将持续运行）
    load_and_run_server("./plugins/libserver_plugin.so", host_context).await;
}
```

## 高级功能

### 1. 使用模板引擎

插件可以通过宿主上下文调用模板渲染功能：

```rust
async fn handle_with_template(req: HttpRequest) -> Result<HttpResponse> {
    let template = r#"
        <h1>欢迎 {{name}}</h1>
        <p>当前时间: {{time}}</p>
    "#;
    
    let data = serde_json::json!({
        "name": "张三",
        "time": chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    });
    
    // 使用宿主提供的模板渲染
    let html = req.host_context.render_template(template, data).await?;
    
    Ok(HttpResponse::html(html))
}
```

### 2. 错误处理

所有的 panic 都会被捕获并转换为错误响应：

```rust
fn may_panic(req: HttpRequest) -> Result<HttpResponse> {
    if req.query.contains_key("panic") {
        panic!("故意触发的 panic!");
    }
    
    Ok(HttpResponse::text("正常响应"))
}
```

### 3. 热重载支持

启用热重载功能后，插件会被复制到临时目录，允许在不停止服务的情况下更新：

```toml
[dependencies]
play-dylib-loader = { path = "../play-dylib-loader", features = ["hot-reload"] }
```

## 跨语言支持

### Go 语言插件

系统提供了 Go 语言的 ABI 绑定，允许使用 Go 开发插件：

```go
package main

import "C"
import (
    "encoding/json"
    "fmt"
)

//export handle_request
func handle_request(requestStr *C.char) *C.char {
    // 解析请求
    var req HttpRequest
    json.Unmarshal([]byte(C.GoString(requestStr)), &req)
    
    // 处理逻辑
    resp := HttpResponse{
        StatusCode: 200,
        Body: fmt.Sprintf("Hello from Go: %s", req.URL),
    }
    
    // 返回响应
    respBytes, _ := json.Marshal(resp)
    return C.CString(string(respBytes))
}

//export free_c_string
func free_c_string(s *C.char) {
    C.free(unsafe.Pointer(s))
}

func main() {}
```

编译 Go 插件：

```bash
go build -buildmode=c-shared -o my_go_plugin.so
```

## 最佳实践

### 1. 内存管理

- 宿主分配请求字符串，插件负责释放
- 插件分配响应字符串，宿主负责释放
- 始终使用提供的 `free_c_string` 函数

### 2. 错误处理

- 使用 `Result` 类型返回可恢复的错误
- 避免 panic，如果必须 panic，加载器会捕获并转换为错误响应
- 在响应中包含有意义的错误信息

### 3. 性能优化

- 插件会被缓存，避免重复加载
- 使用异步处理器处理 I/O 密集型任务
- 大量数据传输时考虑使用流式处理

### 4. 安全考虑

- 插件运行在独立的 tokio 任务中
- 所有数据通过 JSON 序列化传递，避免内存安全问题
- 仔细验证输入数据，避免注入攻击

## 调试技巧

### 1. 启用日志

在插件中使用标准日志库：

```rust
use log::{info, error};

fn handle_request(req: HttpRequest) -> Result<HttpResponse> {
    info!("收到请求: {}", req.url);
    
    // 处理逻辑
    
    Ok(HttpResponse::text("完成"))
}
```

### 2. 测试插件

创建单元测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_handler() {
        let req = HttpRequest {
            // 构建测试请求
        };
        
        let resp = handle_request(req).unwrap();
        assert_eq!(resp.status_code, 200);
    }
}
```

### 3. 使用调试器

可以附加调试器到宿主进程来调试插件代码。

## 常见问题

### Q: 插件加载失败

A: 检查以下几点：
- 确保插件编译为 `cdylib` 类型
- 检查动态库路径是否正确
- 确认导出函数名称正确（使用 `nm` 或 `objdump` 工具）

### Q: 内存泄漏

A: 确保：
- 正确调用 `free_c_string` 释放内存
- 避免在插件中保存全局状态
- 使用 Rust 的所有权系统管理资源

### Q: 性能问题

A: 优化建议：
- 使用插件缓存避免重复加载
- 考虑批处理请求
- 使用异步处理器处理并发请求

## 示例项目

完整的示例代码可以在 `play-dylib-example` crate 中找到，包括：

- 同步和异步处理器示例
- 错误处理示例
- 服务器插件示例
- Panic 处理演示

## 总结

play-dylib 插件系统提供了一个灵活、安全的方式来扩展应用程序功能。通过明确定义的 ABI 接口和强大的加载器，开发者可以轻松创建和集成动态插件，同时保持类型安全和错误处理能力。

无论是简单的请求处理器还是复杂的长时间运行服务，play-dylib 都能满足各种插件开发需求。