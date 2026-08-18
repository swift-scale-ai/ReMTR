# ReMTR 中文使用手册

[English](USER_GUIDE.md) | 简体中文

ReMTR 通过递增 IPv4 TTL 持续探测并显示网络路径。目前只支持原生 Apple Silicon macOS。

## 启动实时界面

```bash
sudo remtr example.com
```

目标会解析为第一个 IPv4 地址。ReMTR 为每个 TTL 发送 ICMP Echo Request；中间路由器的 ICMP Time Exceeded 回复用于识别路径，Echo Reply 表示已经抵达目标。

## 界面布局

- **顶部：**目标、解析地址、发现状态、完成轮次和回复比例；
- **跳点表：**地址、丢包率、发送次数、延迟数据和当前抖动；
- **延迟历史：**当前跳点的实时曲线，未回复样本显示为零高度；
- **路径图：**灰色表示未回复，黄色表示路径发生变化，红色表示高丢包。

延迟低于 60 ms 为绿色，60–149 ms 为黄色，150 ms 及以上为红色。丢包低于 5% 为绿色，5%–19% 为黄色，20% 及以上为红色。所有颜色都保留数字信息。

## 快照模式

```bash
sudo remtr --snapshot --cycles 3 example.com
```

快照模式输出有限轮次的表格后退出。可以通过 `--max-hops`、`--timeout-ms` 和 `--interval-ms` 调整探测。路由器可能限制或忽略 ICMP 回复，因此中间跳点显示丢包不一定代表转发流量真的丢失。

## 当前限制

- 只支持 IPv4 ICMP，后续加入 IPv6、UDP 和 TCP；
- 暂时只显示数字地址，后续加入异步反向 DNS 和 ASN；
- 0.1 版按顺序探测，优先保证正确性；
- macOS raw ICMP 权限可能需要 `sudo`，不要将程序设置为 setuid-root；
- 整个 TUI 通过 `sudo` 运行只是过渡方案，下一安全里程碑是最小化特权 helper。
