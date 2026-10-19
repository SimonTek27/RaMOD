# RaMOD Live Update Protocol

**Version:** 0.1 (S6.1)  
**Transport:** TCP  
**Default port:** `17420`  
**Encoding:** UTF-8, **one JSON object per line** (NDJSON)

## Roles

| Role | Process |
|------|---------|
| **Server** | Game / `ramod_runtime` with feature `live-update` |
| **Client** | `ramod_editor` (or any tool) |

The **game listens**. The **editor connects** outbound.

## Handshake

Client → Server:

```json
{"cmd":"hello","client":"ramod_editor","version":1}\n
```

Server → Client:

```json
{"cmd":"welcome","server":"ramod_runtime","version":1,"features":["param","bus","snapshot"]}\n
```

## Commands (client → server)

### Set event parameter

```json
{"cmd":"set_param","event":"event:/sfx/explosion","name":"Intensity","value":0.8}
```

Applies to **all active instances** of that event path (S6 scope).

### Bus volume / mute / solo

```json
{"cmd":"set_bus","bus":"Master","volume_db":-3.0}
{"cmd":"set_bus","bus":"SFX","mute":true}
{"cmd":"set_bus","bus":"Music","solo":true}
```

Bus is matched by **name** (case-sensitive).

### Snapshot

```json
{"cmd":"snapshot","name":"battle","weight":1.0}
{"cmd":"snapshot_clear"}
```

### Ping

```json
{"cmd":"ping"}
```

Server replies `{"cmd":"pong"}`.

## Server → client notifications (optional)

```json
{"cmd":"status","voices":3,"instances":2}
{"cmd":"error","message":"unknown bus"}
```

## Limits (honest, FMOD-like)

| Supported live | Requires rebuild / restart |
|----------------|----------------------------|
| Parameter values | New event paths |
| Bus volume/mute/solo | New assets / sample data |
| Snapshot blend | Structural track edits |
| | Renames of event paths |

## Security

Bind **localhost only** by default (`127.0.0.1`). No auth in v0.1 — local dev only.
