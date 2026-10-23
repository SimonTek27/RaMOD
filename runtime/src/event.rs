//! Event descriptions and instances.

use std::collections::HashMap;

use crate::buffer::BufferHandle;
use crate::spatial::{Attenuation, Attributes3d};
use crate::parameter::{ParameterDef, ParameterRegistry};
use crate::voice::{PlayParams, VoiceHandle};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventPath(String);

impl EventPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EventPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone)]
pub struct EventDescription {
    pub path: EventPath,
    pub buffer: BufferHandle,
    pub is_3d: bool,
    pub attenuation: Attenuation,
    pub parameters: HashMap<String, f32>,
    /// Optional kind metadata for discrete / labeled params.
    pub param_defs: ParameterRegistry,
    pub priority: i32,
    pub looping: bool,
    pub output_bus: String,
}

impl EventDescription {
    pub fn new(path: impl Into<String>, buffer: BufferHandle) -> Self {
        Self {
            path: EventPath::new(path),
            buffer,
            is_3d: false,
            attenuation: Attenuation::default(),
            parameters: HashMap::new(),
            param_defs: ParameterRegistry::new(),
            priority: 128,
            looping: false,
            output_bus: "SFX".into(),
        }
    }

    pub fn with_3d(mut self, attenuation: Attenuation) -> Self {
        self.is_3d = true;
        self.attenuation = attenuation;
        self
    }

    pub fn with_loop(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_output_bus(mut self, bus: impl Into<String>) -> Self {
        self.output_bus = bus.into();
        self
    }

    pub fn with_parameter(mut self, name: impl Into<String>, default: f32) -> Self {
        let name = name.into();
        self.parameters.insert(name.clone(), default);
        self.param_defs.insert(ParameterDef::continuous(name, default, 0.0, 1.0));
        self
    }

    pub fn with_parameter_def(mut self, def: ParameterDef) -> Self {
        self.parameters.insert(def.name.clone(), def.default);
        self.param_defs.insert(def);
        self
    }
}

#[derive(Debug, Clone)]
pub struct EventInstance {
    pub id: u64,
    pub path: EventPath,
    pub buffer: BufferHandle,
    pub is_3d: bool,
    pub attenuation: Attenuation,
    pub parameters: HashMap<String, f32>,
    pub param_defs: ParameterRegistry,
    pub priority: i32,
    pub looping: bool,
    pub gain_user: f32,
    pub voice: Option<VoiceHandle>,
    pub started: bool,
    pub attributes: Attributes3d,
}

impl EventInstance {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn path(&self) -> &EventPath {
        &self.path
    }

    pub fn set_parameter(&mut self, name: &str, value: f32) -> bool {
        let q = self.param_defs.quantize_value(name, value);
        if let Some(slot) = self.parameters.get_mut(name) {
            *slot = q;
            true
        } else {
            self.parameters.insert(name.to_string(), q);
            true
        }
    }

    pub fn get_parameter(&self, name: &str) -> Option<f32> {
        self.parameters.get(name).copied()
    }

    pub fn set_3d(&mut self, attributes: Attributes3d) {
        self.attributes = attributes;
    }

    pub fn attributes(&self) -> Attributes3d {
        self.attributes
    }

    pub fn is_playing(&self) -> bool {
        self.started && self.voice.is_some()
    }

    pub fn play_params(&self) -> PlayParams {
        PlayParams {
            gain: self.gain_user,
            looped: self.looping,
        }
    }
}

#[derive(Debug)]
pub struct EventSystem {
    next_id: u64,
    descriptions: HashMap<String, EventDescription>,
    instances: HashMap<u64, EventInstance>,
    listener: Attributes3d,
}

impl EventSystem {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            descriptions: HashMap::new(),
            instances: HashMap::new(),
            listener: Attributes3d::default(),
        }
    }

    pub fn register(&mut self, desc: EventDescription) {
        self.descriptions.insert(desc.path.as_str().to_string(), desc);
    }

    pub fn get_description(&self, path: &str) -> Option<&EventDescription> {
        self.descriptions.get(path)
    }

    pub fn set_listener(&mut self, attributes: Attributes3d) {
        self.listener = attributes;
    }

    pub fn listener(&self) -> Attributes3d {
        self.listener
    }

    pub fn create_instance(&mut self, path: &str) -> Option<u64> {
        let desc = self.descriptions.get(path)?;
        let id = self.next_id;
        self.next_id += 1;
        let inst = EventInstance {
            id,
            path: desc.path.clone(),
            buffer: desc.buffer,
            is_3d: desc.is_3d,
            attenuation: desc.attenuation,
            parameters: desc.parameters.clone(),
            param_defs: desc.param_defs.clone(),
            priority: desc.priority,
            looping: desc.looping,
            gain_user: 1.0,
            voice: None,
            started: false,
            attributes: Attributes3d::default(),
        };
        self.instances.insert(id, inst);
        Some(id)
    }

    pub fn instance_mut(&mut self, id: u64) -> Option<&mut EventInstance> {
        self.instances.get_mut(&id)
    }

    pub fn instance(&self, id: u64) -> Option<&EventInstance> {
        self.instances.get(&id)
    }

    pub fn release_instance(&mut self, id: u64) -> Option<EventInstance> {
        self.instances.remove(&id)
    }

    pub fn instances_reinsert(&mut self, inst: EventInstance) {
        self.instances.insert(inst.id, inst);
    }

    pub fn instances_mut(&mut self) -> impl Iterator<Item = &mut EventInstance> {
        self.instances.values_mut()
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}
