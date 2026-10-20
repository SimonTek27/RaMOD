# S5.8 — Bank build from project (editor)

**Status:** on `main`

## Goal

Export a playable bank folder from the editor project so `bevy_ramod` / runtime can load events + asset paths without FMOD.

## Delivered

| Item | Detail |
|------|--------|
| Assign asset → event | `Project::assign_asset_to_event` + editor button |
| Build bank | `Project::export_bank_build` → `build/<BankName>/bank.json` + `assets/` |
| Manifest | JSON with event path, asset file relative path, 3D, params |

## Layout

```
build/
  Master/
    bank.json
    assets/
      footstep.wav
      impact.ogg
```

## Example bank.json

```json
{
  "name": "Master",
  "project": "Untitled",
  "events": [
    {
      "path": "event:/sfx/beep",
      "asset": "footstep",
      "asset_file": "assets/footstep.wav",
      "looping": false,
      "is_3d": false,
      "priority": 128,
      "min_distance": 1.0,
      "max_distance": 50.0,
      "parameters": [["Intensity", 1.0]]
    }
  ]
}
```

## Editor usage

1. Import audio (Assets tab or drag-drop)
2. Select event + asset → **Assign asset to event**
3. File → **Build bank** (or Banks tab button)
4. Output under `./build/<bank>/`

## Follow-up

- `bevy_ramod` asset loader for `bank.json`
- FSB5 binary export when full `src/fsb.rs` is restored
