# S7.25 Nested group buses

## Done on main
- `BusMeterInfo.parent_id`
- `Runtime::create_child_bus` / `set_bus_parent`
- Mixer already had `Bus.parent` + hierarchical `finish`

## Editor (pending full push / local)
- Profiler hierarchy indent + **+Child** → `CreateChildBus`
- `audio_bridge::create_child_bus`

## Full restore still required
```bash
cp artifacts/recovery/core_b.rs runtime/src/runtime_core_b.rs
cp artifacts/recovery/ab.rs     editor/src/audio_bridge.rs
# then re-apply parent_id in list_bus_meters + create_child_bus if needed
```
