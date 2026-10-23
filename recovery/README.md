# Offline recovery blobs

`build.rs` restores truncated sources from these zlib+base64 chunks when a stub is detected.

| Prefix | Target |
|--------|--------|
| `fsb.z*` | `src/fsb.rs` |
| `ab.z*` | `editor/src/audio_bridge.rs` |
| `core_b.z*` | `runtime/src/runtime_core_b.rs` |

Do **not** commit stub/placeholder versions of the targets above.
