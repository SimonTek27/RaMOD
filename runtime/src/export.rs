//! S5.8 — Build / export bank manifest (JSON).
//!
//! FSB5 binary export waits on `ramod` foundation restore; the manifest is the
//! portable intermediate format for the editor Build step.

use crate::designer::DesignedEvent;

/// Serializable bank build product.
#[derive(Debug, Clone)]
pub struct BankManifest {
    pub name: String,
    pub events: Vec<EventManifestEntry>,
}

#[derive(Debug, Clone)]
pub struct EventManifestEntry {
    pub path: String,
    pub length: f32,
    pub is_3d: bool,
    pub priority: i32,
    pub tracks: usize,
    pub instruments: usize,
    pub parameters: Vec<(String, f32)>,
}

impl BankManifest {
    pub fn from_events(name: impl Into<String>, events: &[DesignedEvent]) -> Self {
        let events = events
            .iter()
            .map(|e| {
                let instruments = e.tracks.iter().map(|t| t.instruments.len()).sum();
                EventManifestEntry {
                    path: e.path.clone(),
                    length: e.length,
                    is_3d: e.is_3d,
                    priority: e.priority,
                    tracks: e.tracks.len(),
                    instruments,
                    parameters: e.parameters.clone(),
                }
            })
            .collect();
        Self {
            name: name.into(),
            events,
        }
    }

    /// Minimal JSON (no serde dep in runtime).
    pub fn to_json(&self) -> String {
        let mut s = String::from("{\n");
        s.push_str(&format!("  \"name\": \"{}\",\n", escape(&self.name)));
        s.push_str("  \"events\": [\n");
        for (i, e) in self.events.iter().enumerate() {
            s.push_str("    {\n");
            s.push_str(&format!("      \"path\": \"{}\",\n", escape(&e.path)));
            s.push_str(&format!("      \"length\": {},\n", e.length));
            s.push_str(&format!("      \"is_3d\": {},\n", e.is_3d));
            s.push_str(&format!("      \"priority\": {},\n", e.priority));
            s.push_str(&format!("      \"tracks\": {},\n", e.tracks));
            s.push_str(&format!("      \"instruments\": {},\n", e.instruments));
            s.push_str("      \"parameters\": {");
            for (j, (k, v)) in e.parameters.iter().enumerate() {
                if j > 0 {
                    s.push_str(", ");
                }
                s.push_str(&format!("\"{}\": {}", escape(k), v));
            }
            s.push_str("}\n");
            s.push_str("    }");
            if i + 1 < self.events.len() {
                s.push(',');
            }
            s.push('\n');
        }
        s.push_str("  ]\n}\n");
        s
    }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
