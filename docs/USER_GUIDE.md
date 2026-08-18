# ReMTR User Guide

English | [简体中文](USER_GUIDE.zh-CN.md)

ReMTR continuously probes increasing IPv4 TTL values and displays the responding network path. It currently targets native Apple Silicon macOS only.

## Start a live session

```bash
sudo remtr example.com
```

The target is resolved to its first IPv4 address. A raw ICMP Echo Request is sent for each TTL. ICMP Time Exceeded messages identify intermediate hops; an Echo Reply marks the destination.

## Screen layout

- **Header:** target, resolved address, discovery state, completed cycles, and response ratio.
- **Hop table:** status, address, replies/sent, loss, last/average/P95 latency, jitter, and an inline recent-latency trend. Wide terminals also show best/worst latency, standard deviation, and route-change count.
- **Selected hop:** timeouts, P95, standard deviation, range, jitter, route changes, and known path count.
- **Latency history:** a sparkline for the selected hop; missing replies appear as zero-height samples.
- **Route topology:** a compact path view showing current and previously observed addresses without repeating the table's loss and RTT data. `◆` marks the destination, `↺` a changed route, `!` high loss, and `×` no reply.

Latency is green below 60 ms, yellow from 60 through 149 ms, and red at 150 ms or above. Loss is green below 5%, yellow from 5% through 19%, and red at 20% or above. Numeric values remain visible, so color is never the only signal.

## Snapshot mode

```bash
sudo remtr --snapshot --cycles 3 example.com
```

Snapshot mode prints a finite table and exits. Use `--max-hops`, `--timeout-ms`, and `--interval-ms` to control probing. Be aware that routers may rate-limit or suppress ICMP replies; loss at one intermediate hop does not necessarily mean forwarded traffic is being lost.

## Known limitations

- IPv4 ICMP only; IPv6, UDP, and TCP probes are planned.
- Numeric addresses only; asynchronous reverse DNS and ASN enrichment are planned.
- Probes are sequential in 0.1, prioritizing correctness over scan speed.
- macOS raw ICMP permission may require `sudo`; do not make the binary setuid-root.
- Running the complete TUI through `sudo` is temporary. A minimal privileged helper is the next security milestone.
