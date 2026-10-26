# IP 连通性探针

`probe.py` 仅依赖 Python 3 标准库，已部署在 `8.153.146.11:/opt/ip-probe/probe.py`。

- 每次 `GET http://8.153.146.11:8081/should-change-ip` 实时检测 `ip.zhouzhipeng.com:80` 的 TCP 连通性，连接超时 5 秒。
- 不通立即返回 JSON `true`，通则返回 JSON `false`；每个请求独立检测，没有后台定时任务、失败计数或缓存状态。
- 探针自身不需要 crontab。下方 crontab 仅用于按需定时调用 play 管理接口。
- systemd 服务名为 `ip-probe`，已启用开机启动及进程退出后自动重启。

```sh
systemctl status ip-probe
journalctl -u ip-probe -f
python3 -m unittest discover -s scripts/ip-probe
```

## play 管理接口

`GET /admin/check-if-should-change-ip` 请求上述探针（总超时 10 秒）。只有成功返回 JSON `true` 才复用 `/admin/one-key-change-ip` 的处理函数，启动已有的后台换 IP 任务及其并发保护；false 不执行操作。探针超时、HTTP 错误、非法 JSON 均返回错误，不换 IP。

此路由沿用 play 的管理鉴权。重新构建并发布 play 后，可在 crontab 中调用，例如（按实际地址及已有指纹填写）：

```cron
* * * * * curl --fail --silent --show-error --max-time 20 -H 'X-Browser-Fingerprint: YOUR_EXISTING_FINGERPRINT' 'http://127.0.0.1:YOUR_PLAY_PORT/admin/check-if-should-change-ip'
```

探针仍为 true 时，每次调用都会尝试启动换 IP；正在运行的换 IP 任务由已有并发保护拒绝重复执行。这里未自动添加 crontab。
