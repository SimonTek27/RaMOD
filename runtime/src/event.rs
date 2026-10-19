//! Event descriptions, instances, parameters (Studio-like API).

use std::collections::HashMap;

use crate::buffer::BufferHandle;
use crate::spatial::{Attributes3d, Attenuation};
use crate::voice::{PlayParams, VoiceHandle};

/// Opaque event path, e.g. `event:/sfx/explosion`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventPath(pub String);

impl EventPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for EventPath {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Template for an event (like FMOD EventDescription).
#[derive(Debug, Clone)]
pub struct EventDescription {
    pub path: EventPath,
    pub buffer: BufferHandle,
    pub is_3d: bool,
    pub attenuation: Attenuation,
    pub parameters: HashMap<String, f32>,
    pub priority: i32,
    pub looping: bool,
}

impl EventDescription {
    pub fn new(path: impl Into<String>, buffer: BufferHandle) -> Self {
        Self {
            path: EventPath::new(path),
            buffer,
            is_3d: false,
            attenuation: Attenuation::default(),
            parameters: HashMap::new(),
            priority: 128,
            looping: false,
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

    pub fn with_parameter(mut self, name: impl Into<String>, default: f32) -> Self {
        self.parameters.insert(name.into(), default);
        self
    }
}

/// Live instance of an event (like FMOD EventInstance).
#[derive(Debug)]
pub struct EventInstance {
    pub(crate) id: u64,
    pub(crate) description_path: EventPath,
    pub(crate) buffer: BufferHandle,
    pub(crate) voice: Option<VoiceHandle>,
    pub(crate) parameters: HashMap<String, f32>,
    pub(crate) attributes: Attributes3d,
    pub(crate) is_3d: bool,
    pub(crate) attenuation: Attenuation,
    pub(crate) priority: i32,
    pub(crate) looping: bool,
    pub(crate) started: bool,
    pub(crate) gain_user: f32,
}

impl EventInstance {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn path(&self) -> &EventPath {
        &self.description_path
    }

    pub fn set_parameter(&mut self, name: &str, value: f32) -> bool {
        if let Some(slot) = self.parameters.get_mut(name) {
            *slot = value;
            true
        } else {
            false
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

    pub fn set_gain(&mut self, gain: f32) {
        self.gain_user = gain.max(0.0);
    }

    pub fn is_playing(&self) -> bool {
        self.started && self.voice.is_some()
    }

    pub(crate) fn play_params(&self) -> PlayParams {
        PlayParams {
            gain: self.gain_user,
            looped: self.looping,
        }
    }
}

pub(crate) struct EventSystem {
    descriptions: HashMap<String, EventDescription>,
    instances: HashMap<u64, EventInstance>,
    next_instance_id: u64,
    listener: Attributes3d,
}

impl EventSystem {
    pub fn new() -> Self {
        Self {
            descriptions: HashMap::new(),
            instances: HashMap::new(),
            next_instance_id: 1,
            listener: Attributes3d::default(),
        }
    }

    pub fn register(&mut self, desc: EventDescription) {
        self.descriptions.insert(desc.path.0.clone(), desc);
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
        let id = self.next_instance_id;
        self.next_instance_id += 1;
        let inst = EventInstance {
            id,
            description_path: desc.path.clone(),
            buffer: desc.buffer,
            voice: None,
            parameters: desc.parameters.clone(),
            attributes: Attributes3d::default(),
            is_3d: desc.is_3d,
            attenuation: desc.attenuation,
            priority: desc.priority,
            looping: desc.looping,
            started: false,
            gain_user: 1.0,
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
