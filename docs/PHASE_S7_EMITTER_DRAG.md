# S7.11 — Drag emitter / listener on 3D map

**Status:** on `main`

## Interaction

| Gesture | Effect |
|---------|--------|
| Drag near orange | Move emitter (playing instance) |
| Drag near green **L** | Move listener |
| Drag empty (with emitter) | Move emitter |
| Double-click | Set listener |

World range ±50 m on XZ.

## API

```rust
audio.set_emitter_xz(x, z);
audio.set_listener_xz(x, z);
```

## Verify

```bash
cargo run -p ramod_editor
# ☑ 3D → Play → drag on map → audibility / gain updates
```
