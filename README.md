# ReMTR

[![CI](https://github.com/swiftscalecloud/ReMTR/actions/workflows/ci.yml/badge.svg)](https://github.com/swiftscalecloud/ReMTR/actions/workflows/ci.yml)
[![License: GPL-2.0-or-later](https://img.shields.io/badge/license-GPL--2.0--or--later-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20Apple%20Silicon-black.svg)](docs/USER_GUIDE.md)
[![Rust 1.83+](https://img.shields.io/badge/rust-1.83%2B-orange.svg)](Cargo.toml)

ReMTR is an independent Rust implementation of an MTR-style network path monitor for Apple Silicon Macs. It combines continuous IPv4 ICMP route discovery with a colorful terminal dashboard, per-hop latency history, route-change indicators, and script-friendly reports.

> Current platform: native `aarch64-apple-darwin` on macOS 14 or later.

ReMTR is an early implementation. It is not yet a drop-in replacement for upstream mtr.

## First release features

- Native Darwin raw ICMP probing without invoking `ping`, `traceroute`, or `mtr`
- Per-hop loss, sent count, last/average/best/worst latency, and current jitter
- Color-coded latency and loss severity
- Selected-hop latency sparkline
- Route graph and path-change markers
- Pause, reset, and hop selection controls
- Finite snapshot mode for diagnostics and automation
- Pure Rust statistics and UI, with `unsafe` isolated to the Darwin socket adapter

## Build and run

Requirements: an Apple Silicon Mac, macOS 14 or later, and Rust 1.83 or later.

```bash
cargo build --release
sudo ./target/release/remtr example.com
```

Raw ICMP sockets require elevated privileges on macOS. ReMTR 0.1 runs as the invoking user only when the operating system permits opening that socket; otherwise it prints a direct `sudo` instruction.

For a finite text report:

```bash
sudo ./target/release/remtr --snapshot --cycles 3 example.com
```

Useful options:

```text
--max-hops N       Maximum TTL, 1 through 64
--timeout-ms N     Per-probe timeout
--interval-ms N    Delay between probes
--cycles N         Snapshot probe cycles
```

## Keyboard controls

- `j`, `k`, or arrow keys: select a hop
- `p` or Space: pause/resume probing
- `r`: reset collected statistics
- `q` or Escape: quit

## Security model

The initial version requires the whole process to run through `sudo` when Darwin denies an unprivileged raw ICMP socket. This is intentionally documented as a first-version limitation, not the final security design. The next security milestone is a small privileged probe helper that opens sockets and exchanges validated messages with an unprivileged TUI.

Do not install the current `remtr` executable setuid-root. See [Architecture](docs/ARCHITECTURE.md) for the planned privilege boundary.

## Relationship to mtr

The official [traviscross/mtr](https://github.com/traviscross/mtr) repository is used as a behavioral and compatibility reference. ReMTR has its own Rust implementation and does not embed or execute upstream mtr.

Both projects are distributed under GPL-compatible terms. ReMTR is licensed under GPL-2.0-or-later.

## Documentation

- [User Guide](docs/USER_GUIDE.md) · [简体中文](docs/USER_GUIDE.zh-CN.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Development](docs/DEVELOPMENT.md)

## License

GPL-2.0-or-later. See [LICENSE](LICENSE).
