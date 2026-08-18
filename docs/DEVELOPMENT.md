# Development

## Requirements

- Apple Silicon Mac
- macOS 14 or later
- Rust 1.83.0

## Checks

```bash
cargo fmt -- --check
cargo check --locked
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

Unit tests do not open raw sockets. A local integration smoke test should use a loopback target and the smallest possible probe scope:

```bash
sudo cargo run -- --snapshot --max-hops 1 --timeout-ms 100 --interval-ms 10 127.0.0.1
```

Never run live internet probes in CI. Packet parsing should be tested with deterministic byte fixtures.
