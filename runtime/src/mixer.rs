//! Hierarchical mixer buses, mute/solo, metering, snapshots (S4).

use std::collections::HashMap;

use crate::dsp::{process_effect, EffectState, InsertEffect};

/// Stable bus id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BusId(pub u32);

/// Peak / RMS meter (linear amplitude).
#[derive(Debug, Clone, Copy, Default)]
pub struct Meter {
    pub peak: f32,
    pub rms: f32,
}

impl Meter {
    pub fn update(&mut self, samples: &[f32]) {
        if samples.is_empty() {
            self.peak *= 0.95;
            self.rms *= 0.95;
            return;
        }
        let mut peak = 0.0f32;
        let mut sum_sq = 0.0f32;
        for &s in samples {
            let a = s.abs();
            if a > peak {
                peak = a;
            }
            sum_sq += s * s;
        }
        let rms = (sum_sq / samples.len() as f32).sqrt();
        // Fast attack, slow release
        self.peak = if peak > self.peak {
            peak
        } else {
            self.peak * 0.92 + peak * 0.08
        };
        self.rms = self.rms * 0.9 + rms * 0.1;
    }

    pub fn peak_db(&self) -> f32 {
        if self.peak < 1e-8 {
            -96.0
        } else {
            20.0 * self.peak.log10()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bus {
    pub id: BusId,
    pub name: String,
    pub parent: Option<BusId>,
    pub volume_db: f32,
    pub mute: bool,
    pub solo: bool,
    pub inserts: Vec<InsertEffect>,
    pub meter: Meter,
    /// Working buffer for this bus (filled during mix).
    pub(crate) buffer: Vec<f32>,
    pub(crate) effect_state: Vec<EffectState>,
}

impl Bus {
    pub fn new(id: BusId, name: impl Into<String>, parent: Option<BusId>) -> Self {
        Self {
            id,
            name: name.into(),
            parent,
            volume_db: 0.0,
            mute: false,
            solo: false,
            inserts: Vec::new(),
            meter: Meter::default(),
            buffer: Vec::new(),
            effect_state: Vec::new(),
        }
    }

    pub fn volume_linear(&self) -> f32 {
        if self.mute {
            0.0
        } else {
            10f32.powf(self.volume_db / 20.0)
        }
    }
}

/// Mixer graph: buses + routing of voice output to a leaf bus.
#[derive(Debug)]
pub struct Mixer {
    buses: HashMap<u32, Bus>,
    next_id: u32,
    master_id: BusId,
    /// voice_id → bus_id (default Master).
    voice_route: HashMap<u64, BusId>,
    default_bus: BusId,
}

impl Mixer {
    pub fn new_with_defaults() -> Self {
        let mut m = Self {
            buses: HashMap::new(),
            next_id: 1,
            master_id: BusId(0),
            voice_route: HashMap::new(),
            default_bus: BusId(0),
        };
        let master = Bus::new(BusId(0), "Master", None);
        m.buses.insert(0, master);
        let sfx = m.add_bus("SFX", Some(BusId(0)));
        let _music = m.add_bus("Music", Some(BusId(0)));
        m.default_bus = sfx;
        m
    }

    pub fn master_id(&self) -> BusId {
        self.master_id
    }

    pub fn default_bus(&self) -> BusId {
        self.default_bus
    }

    pub fn add_bus(&mut self, name: impl Into<String>, parent: Option<BusId>) -> BusId {
        let id = BusId(self.next_id);
        self.next_id += 1;
        self.buses.insert(id.0, Bus::new(id, name, parent));
        id
    }

    pub fn bus(&self, id: BusId) -> Option<&Bus> {
        self.buses.get(&id.0)
    }

    pub fn bus_mut(&mut self, id: BusId) -> Option<&mut Bus> {
        self.buses.get_mut(&id.0)
    }

    pub fn buses(&self) -> impl Iterator<Item = &Bus> {
        self.buses.values()
    }

    pub fn set_volume_db(&mut self, id: BusId, db: f32) {
        if let Some(b) = self.buses.get_mut(&id.0) {
            b.volume_db = db.clamp(-80.0, 12.0);
        }
    }

    pub fn set_mute(&mut self, id: BusId, mute: bool) {
        if let Some(b) = self.buses.get_mut(&id.0) {
            b.mute = mute;
        }
    }

    pub fn set_solo(&mut self, id: BusId, solo: bool) {
        if let Some(b) = self.buses.get_mut(&id.0) {
            b.solo = solo;
        }
    }

    pub fn any_solo(&self) -> bool {
        self.buses.values().any(|b| b.solo)
    }

    pub fn add_insert(&mut self, id: BusId, effect: InsertEffect) {
        if let Some(b) = self.buses.get_mut(&id.0) {
            b.inserts.push(effect);
            b.effect_state.push(EffectState::default());
        }
    }

    pub fn route_voice(&mut self, voice_id: u64, bus: BusId) {
        self.voice_route.insert(voice_id, bus);
    }

    pub fn bus_for_voice(&self, voice_id: u64) -> BusId {
        self.voice_route
            .get(&voice_id)
            .copied()
            .unwrap_or(self.default_bus)
    }

    /// Effective linear gain for a bus considering mute/solo and parents.
    pub fn effective_gain(&self, id: BusId) -> f32 {
        let any_solo = self.any_solo();
        let mut gain = 1.0f32;
        let mut cur = Some(id);
        while let Some(cid) = cur {
            let Some(b) = self.buses.get(&cid.0) else {
                break;
            };
            if b.mute {
                return 0.0;
            }
            if any_solo && !self.solo_path(cid) {
                return 0.0;
            }
            gain *= b.volume_linear();
            cur = b.parent;
        }
        gain
    }

    fn solo_path(&self, id: BusId) -> bool {
        let mut cur = Some(id);
        while let Some(cid) = cur {
            let Some(b) = self.buses.get(&cid.0) else {
                return false;
            };
            if b.solo {
                return true;
            }
            cur = b.parent;
        }
        false
    }

    /// Clear leaf buffers; size = frames * channels.
    pub fn prepare(&mut self, frames: usize, channels: usize) {
        let n = frames * channels;
        for b in self.buses.values_mut() {
            b.buffer.resize(n, 0.0);
            b.buffer.fill(0.0);
        }
    }

    /// Accumulate a voice buffer into its routed bus (pre bus gain).
    pub fn accumulate_voice(&mut self, voice_id: u64, voice_out: &[f32]) {
        let bus_id = self.bus_for_voice(voice_id);
        if let Some(b) = self.buses.get_mut(&bus_id.0) {
            let n = b.buffer.len().min(voice_out.len());
            for i in 0..n {
                b.buffer[i] += voice_out[i];
            }
        }
    }

    /// Fold children into parents, apply inserts + gain, write Master to `out`.
    pub fn finish(&mut self, out: &mut [f32], channels: usize, sample_rate: u32) {
        // Process non-master buses first (children before parents).
        // Simple approach: process all non-master, sum into parent, then master.
        let mut order: Vec<u32> = self.buses.keys().copied().filter(|id| *id != 0).collect();
        order.sort_unstable();

        for id in order {
            let parent = self.buses.get(&id).and_then(|b| b.parent);
            // Apply inserts + meter on this bus buffer (clone to avoid borrow issues)
            let (mut buf, inserts, mut states) = {
                let b = self.buses.get_mut(&id).unwrap();
                let buf = std::mem::take(&mut b.buffer);
                let inserts = b.inserts.clone();
                let states = std::mem::take(&mut b.effect_state);
                (buf, inserts, states)
            };
            while states.len() < inserts.len() {
                states.push(EffectState::default());
            }
            for (fx, st) in inserts.iter().zip(states.iter_mut()) {
                process_effect(fx, st, &mut buf, channels, sample_rate);
            }
            let gain = self.effective_gain(BusId(id));
            for s in buf.iter_mut() {
                *s *= gain;
            }
            // Meter
            if let Some(b) = self.buses.get_mut(&id) {
                b.meter.update(&buf);
                b.effect_state = states;
                // Route to parent
                if let Some(pid) = parent {
                    if let Some(p) = self.buses.get_mut(&pid.0) {
                        let n = p.buffer.len().min(buf.len());
                        for i in 0..n {
                            p.buffer[i] += buf[i];
                        }
                    }
                }
                b.buffer = buf;
            }
        }

        // Master
        let master = self.buses.get_mut(&0).unwrap();
        let mut buf = std::mem::take(&mut master.buffer);
        let inserts = master.inserts.clone();
        let mut states = std::mem::take(&mut master.effect_state);
        while states.len() < inserts.len() {
            states.push(EffectState::default());
        }
        for (fx, st) in inserts.iter().zip(states.iter_mut()) {
            process_effect(fx, st, &mut buf, channels, sample_rate);
        }
        let gain = if master.mute {
            0.0
        } else {
            10f32.powf(master.volume_db / 20.0)
        };
        for s in buf.iter_mut() {
            *s *= gain;
        }
        master.meter.update(&buf);
        master.effect_state = states;

        let n = out.len().min(buf.len());
        out[..n].copy_from_slice(&buf[..n]);
        if n < out.len() {
            out[n..].fill(0.0);
        }
        master.buffer = buf;
    }
}

/// Named mixer snapshot (bus volume targets).
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub name: String,
    /// bus id → volume_db
    pub volumes: HashMap<u32, f32>,
}

impl Snapshot {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            volumes: HashMap::new(),
        }
    }

    pub fn set_volume(&mut self, bus: BusId, db: f32) {
        self.volumes.insert(bus.0, db);
    }
}

/// Snapshot bank + blend weight toward a target snapshot.
#[derive(Debug, Default)]
pub struct SnapshotPlayer {
    snapshots: HashMap<String, Snapshot>,
    /// Current blend: name + weight 0..1 applied on top of base volumes.
    active: Option<(String, f32)>,
    base_volumes: HashMap<u32, f32>,
}

impl SnapshotPlayer {
    pub fn add(&mut self, snap: Snapshot) {
        self.snapshots.insert(snap.name.clone(), snap);
    }

    pub fn capture_base(&mut self, mixer: &Mixer) {
        self.base_volumes.clear();
        for b in mixer.buses() {
            self.base_volumes.insert(b.id.0, b.volume_db);
        }
    }

    pub fn trigger(&mut self, name: &str, weight: f32) {
        if self.snapshots.contains_key(name) {
            self.active = Some((name.to_string(), weight.clamp(0.0, 1.0)));
        }
    }

    pub fn clear(&mut self) {
        self.active = None;
    }

    /// Apply blended volumes onto mixer.
    pub fn apply(&self, mixer: &mut Mixer) {
        let Some((ref name, w)) = self.active else {
            return;
        };
        let Some(snap) = self.snapshots.get(name) else {
            return;
        };
        for (id, target) in &snap.volumes {
            let base = self.base_volumes.get(id).copied().unwrap_or(0.0);
            let db = base + (target - base) * w;
            mixer.set_volume_db(BusId(*id), db);
        }
    }
}
