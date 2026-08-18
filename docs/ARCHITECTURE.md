# Architecture

ReMTR is intentionally layered so protocol access, statistics, and presentation can evolve independently.

```text
CLI / TUI
   │
PathModel (loss, latency, jitter, history, route changes)
   │
Probe worker (scheduling, pause/stop, event channel)
   │
Darwin adapter (raw socket, TTL, ICMP encoding/decoding)
```

## Safety boundary

All direct BSD socket calls and all `unsafe` blocks live in `src/platform/darwin.rs`. The adapter owns its file descriptor, validates packet lengths before parsing, matches both identifier and sequence, and closes the descriptor in `Drop`.

The initial process-level privilege model is deliberately conservative and simple: the user invokes ReMTR with `sudo` if raw socket creation is denied. The executable must not be installed setuid-root.

The planned design separates `remtr-probe` from `remtr`. The helper will open only the required sockets, drop unnecessary privileges, validate a small versioned IPC protocol, and expose structured probe results. DNS, statistics, rendering, exports, and user input will remain unprivileged.

## Compatibility reference

Upstream mtr is consulted for observable behavior, packet edge cases, and field semantics. ReMTR does not link to upstream C code and does not reproduce its curses architecture. Protocol behavior is implemented in Rust against Darwin's BSD socket API.
