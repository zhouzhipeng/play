# play-mcp

MCP (Model Context Protocol) 客户端，提供磁盘空间监控功能，可接入小智AI等支持MCP协议的服务。

## 功能特性

- 作为 MCP 客户端连接到外部 MCP 服务器（如小智AI）
- 提供磁盘空间监控工具
- 支持配置文件管理
- 自动重连机制
- JSON-RPC 2.0 协议实现

## 配置文件

编辑 `config.json` 文件配置 MCP 接入点：

```json
{
  "mcp_server": {
    "url": "ws://localhost:8765",  // 小智AI MCP 接入点 WebSocket URL
    "comment": "小智AI MCP 接入点 WebSocket URL"
  },
  "client": {
    "name": "play-mcp-disk-monitor",
    "version": "0.1.0",
    "description": "磁盘空间监控 MCP 客户端"
  },
  "retry": {
    "enabled": true,
    "interval_seconds": 5,
    "max_attempts": 0  // 0 表示无限重试
  }
}
```

## 使用方法

### 使用默认配置文件

```bash
cargo run
```

### 指定配置文件

```bash
cargo run -- --config /path/to/config.json
```

### 命令行参数覆盖配置

```bash
# 覆盖 WebSocket URL
cargo run -- --url ws://ai.example.com:8080

# 覆盖客户端名称
cargo run -- --name my-disk-monitor

# 同时覆盖多个参数
cargo run -- --url ws://ai.example.com:8080 --name my-monitor
```

## 提供的工具

### get_disk_space

获取磁盘空间信息。

**参数：**
- `path` (可选): 要检查的路径。如果不提供，返回所有磁盘的信息。

**返回值：**
- `path`: 挂载点路径
- `total_gb`: 总磁盘空间 (GB)
- `available_gb`: 可用磁盘空间 (GB)
- `used_gb`: 已用磁盘空间 (GB)
- `used_percentage`: 磁盘使用百分比

## MCP 协议流程

1. **连接**: 客户端连接到配置的 WebSocket 服务器
2. **初始化**: 发送 `initialize` 请求，包含客户端信息
3. **注册工具**: 通过 `notifications/tools/list` 注册可用工具
4. **处理请求**: 响应服务器发送的 `tools/call` 请求
5. **保持连接**: 处理 ping/pong 保持连接活跃

## 接入小智AI

1. 获取小智AI的 MCP 接入点 WebSocket URL
2. 修改 `config.json` 中的 `mcp_server.url` 为小智AI提供的地址
3. 运行客户端：`cargo run`
4. 客户端会自动连接并注册工具，小智AI即可调用磁盘空间监控功能

## 日志输出

程序使用 `tracing` 库输出日志，可通过环境变量控制日志级别：

```bash
RUST_LOG=debug cargo run  # 输出详细调试信息
RUST_LOG=info cargo run   # 输出常规信息（默认）
RUST_LOG=error cargo run  # 仅输出错误
```

## 开发说明

### 添加新工具

1. 在 `handle_server_request` 函数中添加新的工具处理逻辑
2. 在注册工具部分添加工具定义
3. 实现具体的工具功能函数

### 测试

可以使用任何支持 MCP 协议的服务器进行测试，或使用提供的 Python 测试服务器脚本。