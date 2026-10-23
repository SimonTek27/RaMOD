# S7.26 Discrete / Labeled parameters

## Runtime (`ramod_runtime::parameter`)

| Kind | Behaviour |
|------|-----------|
| **Continuous** | clamp `[min, max]` |
| **Discrete** | round → `i32` in `[min_i, max_i]` |
| **Labeled** | value = label index; `label_for()` |

```rust
use ramod_runtime::{ParameterDef, EventDescription};

let desc = EventDescription::new("event:/footstep", buf)
    .with_parameter_def(ParameterDef::labeled(
        "Surface",
        vec!["Dirt".into(), "Grass".into(), "Metal".into()],
        0,
    ))
    .with_parameter_def(ParameterDef::discrete("Variant", 0, 0, 3));

inst.set_parameter("Surface", 1.7); // → 2.0 → "Metal"
```

## Project

- `ParameterKindEntry` / `ParameterDefEntry` on `EventEntry.parameter_defs`
- Demo: **Surface** = Dirt | Grass | Metal
- Legacy `parameters: Vec<(String, f32)>` kept for values

## Tests

Unit tests in `parameter.rs`.

## Next

- Event Editor ComboBox for labeled / integer DragValue for discrete
- Live Update: optional label string alongside index
