//! Parameter definitions: continuous, discrete, labeled (FMOD-style).

use std::collections::HashMap;

/// How a parameter value is interpreted and clamped.
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub enum ParameterKind {
    /// Free float in `[min, max]`.
    #[default]
    Continuous,
    /// Integer steps from `min_i` ..= `max_i` (stored as f32).
    Discrete { min_i: i32, max_i: i32 },
    /// Enum-like: value is label index `0 .. labels.len()`.
    Labeled { labels: Vec<String> },
}

/// Full parameter definition (authoring + runtime).
#[derive(Debug, Clone)]
pub struct ParameterDef {
    pub name: String,
    pub default: f32,
    pub min: f32,
    pub max: f32,
    pub kind: ParameterKind,
}

impl ParameterDef {
    pub fn continuous(name: impl Into<String>, default: f32, min: f32, max: f32) -> Self {
        Self {
            name: name.into(),
            default: default.clamp(min, max),
            min,
            max,
            kind: ParameterKind::Continuous,
        }
    }

    pub fn discrete(name: impl Into<String>, default: i32, min_i: i32, max_i: i32) -> Self {
        let min_i = min_i.min(max_i);
        let max_i = max_i.max(min_i);
        let d = default.clamp(min_i, max_i);
        Self {
            name: name.into(),
            default: d as f32,
            min: min_i as f32,
            max: max_i as f32,
            kind: ParameterKind::Discrete { min_i, max_i },
        }
    }

    pub fn labeled(name: impl Into<String>, labels: Vec<String>, default_index: usize) -> Self {
        let n = labels.len().max(1);
        let idx = default_index.min(n - 1);
        Self {
            name: name.into(),
            default: idx as f32,
            min: 0.0,
            max: (n - 1) as f32,
            kind: ParameterKind::Labeled { labels },
        }
    }

    /// Clamp / quantize a raw value according to kind.
    pub fn quantize(&self, value: f32) -> f32 {
        quantize(&self.kind, value, self.min, self.max)
    }

    /// Human-readable label for the current value (labeled params).
    pub fn label_for(&self, value: f32) -> Option<&str> {
        match &self.kind {
            ParameterKind::Labeled { labels } => {
                let i = value.round() as usize;
                labels.get(i).map(|s| s.as_str())
            }
            _ => None,
        }
    }
}

pub fn quantize(kind: &ParameterKind, value: f32, min: f32, max: f32) -> f32 {
    match kind {
        ParameterKind::Continuous => value.clamp(min, max),
        ParameterKind::Discrete { min_i, max_i } => {
            let v = value.round() as i32;
            v.clamp(*min_i, *max_i) as f32
        }
        ParameterKind::Labeled { labels } => {
            let n = labels.len().max(1) as i32;
            let i = value.round() as i32;
            i.clamp(0, n - 1) as f32
        }
    }
}

/// Registry of parameter definitions keyed by name.
#[derive(Debug, Clone, Default)]
pub struct ParameterRegistry {
    defs: HashMap<String, ParameterDef>,
}

impl ParameterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, def: ParameterDef) {
        self.defs.insert(def.name.clone(), def);
    }

    pub fn get(&self, name: &str) -> Option<&ParameterDef> {
        self.defs.get(name)
    }

    pub fn quantize_value(&self, name: &str, value: f32) -> f32 {
        match self.defs.get(name) {
            Some(d) => d.quantize(value),
            None => value,
        }
    }

    pub fn defaults_map(&self) -> HashMap<String, f32> {
        self.defs
            .iter()
            .map(|(k, d)| (k.clone(), d.default))
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ParameterDef> {
        self.defs.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuous_clamp() {
        let d = ParameterDef::continuous("Gain", 0.5, 0.0, 1.0);
        assert!((d.quantize(1.5) - 1.0).abs() < 1e-6);
        assert!((d.quantize(-0.2) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn discrete_steps() {
        let d = ParameterDef::discrete("Variant", 1, 0, 3);
        assert_eq!(d.quantize(1.6), 2.0);
        assert_eq!(d.quantize(9.0), 3.0);
    }

    #[test]
    fn labeled_index() {
        let d = ParameterDef::labeled(
            "Surface",
            vec!["Dirt".into(), "Grass".into(), "Metal".into()],
            0,
        );
        assert_eq!(d.quantize(1.4), 1.0);
        assert_eq!(d.label_for(1.0), Some("Grass"));
        assert_eq!(d.quantize(99.0), 2.0);
    }
}
