# DomainProxy 配置文档

DomainProxy 是 play-server 中的域名代理功能，支持将特定域名代理到本地目录（静态文件服务）或上游服务器（HTTP反向代理）。它提供完整的HTTP和WebSocket代理支持。

## 配置格式

在 `config.toml` 中，DomainProxy 使用数组配置，可以配置多个域名代理规则：

```toml
[[domain_proxy]]
# 代理配置...
```

## 支持的代理类型

### 1. 目录代理 (folder)

将域名代理到本地目录，用于提供静态文件服务。

**配置格式：**
```toml
[[domain_proxy]]
proxy_domain = "static.example.com"
type = "folder"
folder_path = "/path/to/static/files"
```

**字段说明：**
- `proxy_domain`: 要代理的域名
- `type`: 代理类型，设置为 `"folder"`
- `folder_path`: 本地目录路径

### 2. 上游代理 (upstream)

将域名通过HTTP反向代理转发到上游服务器，支持完整的HTTP和WebSocket协议。

**配置格式：**

#### 使用默认配置
```toml
[[domain_proxy]]
proxy_domain = "api.example.com"
type = "upstream"
# ip 默认: 127.0.0.1
# port 默认: 80
```

#### 指定端口
```toml
[[domain_proxy]]
proxy_domain = "service.example.com"
type = "upstream"
port = 8080
# ip 默认: 127.0.0.1
```

#### 指定IP和端口
```toml
[[domain_proxy]]
proxy_domain = "remote.example.com"
type = "upstream"
ip = "192.168.1.100"
port = 3000
```

#### 配置WebSocket支持
```toml
[[domain_proxy]]
proxy_domain = "websocket.example.com"
type = "upstream"
ip = "127.0.0.1"
port = 3000

# WebSocket配置（可选，直接在domain_proxy下配置）
origin_strategy = "backend"  # 或 "keep", "remove", "host", "custom"
# custom_origin = "https://custom-origin.com"  # 仅当origin_strategy为"custom"时需要

# 强制使用HTTPS（可选，默认443端口使用https，其他端口使用http）
# use_https = true
```

**字段说明：**
- `proxy_domain`: 要代理的域名
- `type`: 代理类型，设置为 `"upstream"`
- `ip`: 目标服务器IP地址（默认: `127.0.0.1`）
- `port`: 目标服务器端口（默认: `80`）
- `use_https`: 强制使用HTTPS协议（可选，默认443端口自动使用https）
- `origin_strategy`: WebSocket Origin处理策略（可选）
- `custom_origin`: 自定义Origin值（可选）

## WebSocket配置详解

WebSocket配置允许你控制WebSocket连接的Origin头部处理方式，解决"Invalid origin"等跨域问题。

### WebSocket配置字段

```toml
[[domain_proxy]]
# ... 其他配置 ...
origin_strategy = "backend"  # Origin处理策略
custom_origin = "https://example.com"  # 自定义Origin（可选）
```

### Origin处理策略

#### 1. `"keep"` - 保持原始Origin
```toml
[[domain_proxy]]
# ... 其他配置 ...
origin_strategy = "keep"
```
保持客户端发送的原始Origin头部不变。适用于目标服务器接受任意Origin的情况。

#### 2. `"remove"` - 移除Origin头部
```toml
[[domain_proxy]]
# ... 其他配置 ...
origin_strategy = "remove"
```
完全移除Origin头部，让目标服务器跳过Origin检查。适用于目标服务器不进行Origin验证的情况。

#### 3. `"host"` - 使用代理域名作为Origin
```toml
[[domain_proxy]]
# ... 其他配置 ...
origin_strategy = "host"
```
将Origin设置为代理域名（如 `https://websocket.example.com`）。适用于目标服务器期望Origin与访问域名一致的情况。

#### 4. `"backend"` - 使用后端服务器地址作为Origin（默认）
```toml
[[domain_proxy]]
# ... 其他配置 ...
origin_strategy = "backend"
```
将Origin设置为后端服务器地址（如 `http://127.0.0.1:3000`）。这是默认策略，适用于大多数情况。

#### 5. `"custom"` - 使用自定义Origin
```toml
[[domain_proxy]]
# ... 其他配置 ...
origin_strategy = "custom"
custom_origin = "https://trusted-origin.com"
```
使用完全自定义的Origin值。适用于目标服务器只接受特定Origin的情况。

## 完整配置示例

```toml
# 基础服务配置
[database]
url = "sqlite://data.db"

server_port = 3000
log_level = "INFO"

# 域名代理配置
[[domain_proxy]]
proxy_domain = "files.mysite.com"
type = "folder"
folder_path = "/var/www/static"

[[domain_proxy]]
proxy_domain = "api.mysite.com"  
type = "upstream"
port = 8080

[[domain_proxy]]
proxy_domain = "backend.mysite.com"
type = "upstream"
ip = "10.0.0.10"
port = 9000

[[domain_proxy]]
proxy_domain = "docs.mysite.com"
type = "folder"
folder_path = "/usr/share/docs"

# WebSocket应用代理（使用默认配置）
[[domain_proxy]]
proxy_domain = "websocket.mysite.com"
type = "upstream"
ip = "127.0.0.1"
port = 8080

# WebSocket应用代理（自定义Origin处理）
[[domain_proxy]]
proxy_domain = "chat.mysite.com"
type = "upstream"
ip = "127.0.0.1"
port = 9000
origin_strategy = "remove"

# 需要特定Origin的WebSocket服务
[[domain_proxy]]
proxy_domain = "secure-websocket.mysite.com"
type = "upstream"
ip = "192.168.1.100"
port = 3000
origin_strategy = "custom"
custom_origin = "https://trusted-app.mysite.com"

# 外部HTTPS服务代理（强制使用HTTPS）
[[domain_proxy]]
proxy_domain = "external-service.mysite.com"
type = "upstream"
ip = "192.168.1.100"
port = 3000
use_https = true
origin_strategy = "host"
```

## 工作原理

### 目录代理 (folder)
- 接收到指定域名的请求时，从配置的本地目录提供静态文件服务
- 支持常见的Web文件格式（HTML、CSS、JS、图片等）
- 自动处理MIME类型和缓存头

### 上游代理 (upstream)
- 使用HTTP反向代理技术，完整转发HTTP/WebSocket请求到目标服务器
- **协议支持**: 
  - HTTP/1.1 和 HTTP/2 协议
  - WebSocket 协议自动检测和升级
  - 分块传输编码 (Transfer-Encoding: chunked)
  - Keep-Alive 连接
- **WebSocket支持**:
  - 自动检测WebSocket升级请求
  - 可配置的Origin头部处理策略
  - 支持长连接和双向通信
- **错误处理**: 自动检测连接错误并返回适当的HTTP状态码
- **日志记录**: 详细的请求和WebSocket连接日志

## 性能特性

- **异步处理**: 完全异步I/O，不阻塞其他请求
- **反向代理**: 基于axum-reverse-proxy，提供高性能的HTTP和WebSocket代理
- **智能路由**: 根据Host头自动路由到对应的代理配置
- **协议升级**: 自动处理WebSocket协议升级，无需额外配置
- **错误恢复**: 自动检测和处理代理错误，返回适当的错误响应

## 使用场景

1. **静态资源CDN**: 使用目录代理提供静态文件服务
2. **API网关**: 使用上游代理转发API请求到后端服务
3. **WebSocket代理**: 代理WebSocket连接，支持实时通信应用
4. **微服务路由**: 根据域名路由请求到不同的微服务实例
5. **开发环境**: 本地开发时代理不同域名到不同的开发服务器
6. **跨域解决**: 通过代理解决WebSocket跨域和Origin验证问题
7. **协议升级**: 无缝处理HTTP到WebSocket的协议升级

## 注意事项

1. **域名匹配**: 代理基于HTTP Host头进行匹配，确保客户端发送正确的Host头
2. **WebSocket Origin**: 根据目标服务器的要求选择合适的Origin处理策略
3. **协议选择**: 端口443自动使用HTTPS，其他端口使用HTTP
4. **安全考虑**: 目录代理只能访问配置的目录及其子目录
5. **性能优化**: 建议将频繁访问的静态资源使用目录代理而不是上游代理
6. **日志监控**: 查看日志来调试WebSocket连接问题和Origin配置

## 故障排除

### WebSocket连接失败
如果遇到"Invalid origin"错误，尝试以下Origin策略：
1. 首先尝试 `"remove"` 策略
2. 如果不行，尝试 `"backend"` 策略
3. 最后考虑使用 `"custom"` 策略指定目标服务器期望的Origin

### 查看详细日志
启用INFO级别日志可以看到详细的代理和WebSocket处理过程：
```toml
log_level = "INFO"
```