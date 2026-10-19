//! Simple bank registry: path → event description (S2.7).
//!
//! Full FSB5 loading waits on `ramod` crate restore; S2 registers events from
//! already-loaded PCM buffers.

use std::collections::HashMap;

use crate::buffer::BufferHandle;
use crate::event::EventDescription;
use crate::spatial::Attenuation;

/// Logical bank name (e.g. `Master`, `SFX`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BankName(pub String);

impl BankName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// One entry registered into the runtime event system.
#[derive(Debug, Clone)]
pub struct BankEventSpec {
    pub path: String,
    pub buffer: BufferHandle,
    pub is_3d: bool,
    pub looping: bool,
    pub priority: i32,
    pub min_distance: f32,
    pub max_distance: f32,
}

impl BankEventSpec {
    pub fn new(path: impl Into<String>, buffer: BufferHandle) -> Self {
        Self {
            path: path.into(),
            buffer,
            is_3d: false,
            looping: false,
            priority: 128,
            min_distance: 1.0,
            max_distance: 50.0,
        }
    }

    pub fn into_description(self) -> EventDescription {
        let mut d = EventDescription::new(self.path, self.buffer)
            .with_loop(self.looping)
            .with_priority(self.priority);
        if self.is_3d {
            d = d.with_3d(Attenuation {
                min_distance: self.min_distance,
                max_distance: self.max_distance,
            });
        }
        d
    }
}

/// Collection of event specs under a bank name.
#[derive(Debug, Default)]
pub struct Bank {
    pub name: BankName,
    pub events: Vec<BankEventSpec>,
}

impl Bank {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: BankName::new(name),
            events: Vec::new(),
        }
    }

    pub fn add_event(&mut self, spec: BankEventSpec) {
        self.events.push(spec);
    }
}

/// Loaded banks (metadata only; audio lives in Runtime buffers).
#[derive(Debug, Default)]
pub struct BankRegistry {
    banks: HashMap<String, Bank>,
}

impl BankRegistry {
    pub fn new() -> Self {
        Self {
            banks: HashMap::new(),
        }
    }

    pub fn insert(&mut self, bank: Bank) {
        self.banks.insert(bank.name.0.clone(), bank);
    }

    pub fn get(&self, name: &str) -> Option<&Bank> {
        self.banks.get(name)
    }
}
