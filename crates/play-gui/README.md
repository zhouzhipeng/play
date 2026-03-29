# play-gui

统一入口桌面应用，用来打开 Play 相关工具。

当前行为：

- 启动并打开 `play-server`
- 自动发现 `crates/play-gui/*` 下的工具 crate
- 通过同一个桌面窗口统一打开这些工具
