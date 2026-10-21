# S6 — Live Update end-to-end

**Status:** on `main`

## Roles

| Process | Role | Port |
|---------|------|------|
| Bevy game (`RamodPlugin { enable_live_update: true }`) | **Server** | `127.0.0.1:17420` |
| `ramod_editor` | **Client** | connects outbound |

## Run

```bash
# Terminal 1 — game listens
cargo run -p bevy_ramod --example live_update

# Terminal 2 — editor
cargo run -p ramod_editor
# Status bar → click "Live Update Off" → On
# Select event, drag Intensity slider → set_param over TCP
```

## What is live

- Event parameter values (`set_param`)
- Bus volume / mute / solo
- Snapshot trigger / clear

## What needs rebuild

- New event paths, sample data, structural track edits

## Protocol

See `docs/LIVE_UPDATE_PROTOCOL.md` (NDJSON over TCP).

## API

```rust
App::new().add_plugins(RamodPlugin {
    open_device: true,
    enable_live_update: true, // default true
});
```

`LiveUpdateHub` resource holds the optional `LiveUpdateServer`; polled each frame in `PostUpdate`.
