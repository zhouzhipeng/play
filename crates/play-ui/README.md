# play-ui

Play Server 的用户界面组件，提供系统托盘功能。

## 功能特性

- 🚀 **启动时自动打开首页** - 服务器启动时自动在默认浏览器中打开首页
- 🔧 **系统托盘图标** - 在系统托盘/菜单栏显示图标
- 📋 **托盘菜单** - 右键点击托盘图标显示菜单
  - 打开首页：在浏览器中打开服务器首页
  - 退出：关闭服务器和 UI

## 平台支持

- ✅ macOS - 在菜单栏显示图标
- ✅ Windows - 在系统托盘显示图标  
- ✅ Linux - 在系统托盘显示图标（需要 GTK）

## 使用方式

### 在 play-server 中使用

```rust
#[cfg(feature = "play-ui")]
{
    // 初始化 UI 系统
    play_ui::init();
    
    // 启动服务器
    tokio::spawn(async move {
        start_server(router, app_state).await.expect("Failed to start server");
    });
    
    // 启动 UI（包括系统托盘）
    play_ui::start_window(&format!("http://127.0.0.1:{}", server_port))
        .expect("Failed to start UI");
}
```

### 编译和运行

```bash
# 编译带 UI 的服务器
cargo build -p play-server --features play-ui

# 运行
cargo run -p play-server --features play-ui
```

## 实现细节

### 核心结构

- `TrayApp` - 管理系统托盘图标和菜单
- 使用 `tray-icon` crate 实现跨平台托盘功能
- 使用 `webbrowser` crate 打开默认浏览器

### macOS 特殊处理

在 macOS 上，应用设置为 "Accessory" 模式：
- 不在 Dock 显示图标
- 只在菜单栏显示托盘图标
- 适合后台服务类应用

### 图标

- 默认使用 `/crates/play-ui/icon.png` 作为托盘图标
- 如果文件不存在，会生成一个渐变色的默认图标
- 图标大小：32x32 像素

## 依赖

- `tray-icon` - 系统托盘功能
- `webbrowser` - 打开浏览器
- `image` - 图标处理
- `cocoa` (macOS) - macOS 原生 API
- `winapi` (Windows) - Windows 原生 API
- `gtk` (Linux) - Linux GTK 支持

## 行为说明

1. **启动流程**：
   - 初始化平台特定设置
   - 打开默认浏览器访问服务器首页
   - 创建系统托盘图标
   - 显示提示信息
   - 进入事件循环

2. **菜单操作**：
   - "打开首页" - 在浏览器中打开服务器 URL
   - "退出" - 关闭服务器和 UI

3. **退出处理**：
   - 点击退出菜单项会设置退出标志
   - 事件循环检测到退出标志后结束程序