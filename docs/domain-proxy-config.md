# DomainProxy 配置文档

DomainProxy 是 play-server 中的域名代理功能，支持将特定域名代理到本地目录（静态文件服务）或上游服务器（TCP隧道）。

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

将域名通过TCP隧道代理到上游服务器，支持完整的TCP转发和连接复用。

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

**字段说明：**
- `proxy_domain`: 要代理的域名
- `type`: 代理类型，设置为 `"upstream"`
- `ip`: 目标服务器IP地址（默认: `127.0.0.1`）
- `port`: 目标服务器端口（默认: `80`）

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

[[domain_proxy]]
proxy_domain = "external-service.mysite.com"
type = "upstream"
ip = "192.168.1.100"
port = 3000
```

## 工作原理

### 目录代理 (folder)
- 接收到指定域名的请求时，从配置的本地目录提供静态文件服务
- 支持常见的Web文件格式（HTML、CSS、JS、图片等）
- 自动处理MIME类型和缓存头

### 上游代理 (upstream)
- 使用TCP隧道技术，完整转发HTTP请求到目标服务器
- **连接复用**: 对相同目标的多个请求共享TCP连接，提高性能
- **协议支持**: 完整支持HTTP/1.1协议特性
  - Content-Length 响应
  - 分块传输编码 (Transfer-Encoding: chunked)
  - Keep-Alive 连接
- **错误处理**: 自动检测和恢复失效连接
- **超时控制**: 30秒请求超时，防止连接挂起

## 性能特性

- **异步处理**: 完全异步I/O，不阻塞其他请求
- **连接池**: 全局TCP连接池，支持连接复用和管理
- **智能路由**: 根据Host头自动路由到对应的代理配置
- **错误恢复**: 自动重建失效连接，保证服务可用性

## 使用场景

1. **静态资源CDN**: 使用目录代理提供静态文件服务
2. **API网关**: 使用上游代理转发API请求到后端服务
3. **微服务路由**: 根据域名路由请求到不同的微服务实例
4. **负载均衡**: 结合多个upstream配置实现简单负载均衡
5. **开发环境**: 本地开发时代理不同域名到不同的开发服务器

## 注意事项

1. **域名匹配**: 代理基于HTTP Host头进行匹配，确保客户端发送正确的Host头
2. **连接管理**: 上游代理使用连接池，长时间无活动的连接会被自动清理
3. **安全考虑**: 目录代理只能访问配置的目录及其子目录
4. **性能优化**: 建议将频繁访问的静态资源使用目录代理而不是上游代理