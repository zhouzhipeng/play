# play-gui

统一入口桌面应用，用来在同一个 `egui` 进程里打开 Play 相关工具。

当前行为：

- 不再启动 `play-server`
- 不再通过子进程拉起工具
- 直接以内嵌库的方式承载工具界面
- toolbox 主窗口会一直保留
- 每个工具会在独立原生窗口中打开
- 当前内嵌工具包括 `curl-helper` 和 `frp-client`

打包 macOS DMG：

```bash
bash crates/play-gui/scripts/build_dmg.sh
```
