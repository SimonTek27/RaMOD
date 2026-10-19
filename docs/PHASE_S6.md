# Phase S6 — Live Update

**Status:** On `main` (2026-10-19)

## Goal

Edit parameters / mix while the game (or runtime demo) runs.

## Done

| ID | Item |
|----|------|
| S6.1 | Protocol: [LIVE_UPDATE_PROTOCOL.md](LIVE_UPDATE_PROTOCOL.md) |
| S6.2 | Feature `live-update` + `LiveUpdateServer` |
| S6.3 | Editor client; status bar On when connected |
| S6.4 | Example `live_update_demo` (TCP client smoke) |

## Runtime

```toml
# runtime/Cargo.toml
live-update = []  # std only
```

```rust
#[cfg(feature = "live-update")]
{
    let mut server = LiveUpdateServer::bind("127.0.0.1:17420")?;
    // each frame:
    server.poll(&mut runtime)?;
}
```

## Editor

Click **Live Update Off** in the status bar to connect to `127.0.0.1:17420`.
Intensity slider sends `set_param` when connected.

## Test

```bash
# Terminal A — host a silent runtime with server (example)
cargo run -p ramod_runtime --example live_update_demo --features live-update

# Terminal B — editor
cargo run -p ramod_editor
# Click "Live Update Off" → should become On
```
