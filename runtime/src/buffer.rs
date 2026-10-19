//! PCM buffer storage.

use std::collections::HashMap;

/// Handle to a loaded PCM buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle(pub(crate) u64);

/// Interleaved f32 PCM in memory.
#[derive(Debug, Clone)]
pub struct PcmBuffer {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: u32,
}

impl PcmBuffer {
    pub fn frames(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / self.channels as usize
        }
    }
}

pub(crate) struct BufferStore {
    next: u64,
    map: HashMap<u64, PcmBuffer>,
}

impl BufferStore {
    pub fn new() -> Self {
        Self {
            next: 1,
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, buf: PcmBuffer) -> BufferHandle {
        let id = self.next;
        self.next += 1;
        self.map.insert(id, buf);
        BufferHandle(id)
    }

    pub fn get(&self, h: BufferHandle) -> Option<&PcmBuffer> {
        self.map.get(&h.0)
    }
}
