//! Hierarchical mixer with bus meters, voice routing, and inserts.

use std::collections::HashMap;

use crate::dsp::{process_sidechain, EffectState, InsertEffect};

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
            self.peak *= 0.9;
            self.rms *= 0.9;
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
        self.peak = peak.max(self.peak * 0.85);
        self.rms = (sum_sq / samples.len() as f32).sqrt();
    }

    pub fn peak_db(&self) -> f32 {
        if self.peak <= 1e-9 {
            -96.0
        } else {
            20.0 * self.peak.log10()
        }
    }
}

#[derive(Debug)]
pub struct Bus {
    pub id: BusId,
    pub name: String,
    pub parent: Option<BusId>,
    pub volume_db: f32,
    pub mute: bool,
    pub solo: bool,
    pub inserts: Vec<InsertEffect>,
    pub meter: Meter,
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

#[derive(Debug)]
pub struct Mixer {
    buses: HashMap<u32, Bus>,
    next_id: u32,
    master_id: BusId,
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
        m.buses.insert(0, Bus::new(BusId(0), "Master", None));
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

    pub fn find_by_name(&self, name: &str) -> Option<BusId> {
        self.buses
            .values()
            .find(|b| b.name.eq_ignore_ascii_case(name))
            .map(|b| b.id)
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

    pub fn clear_inserts(&mut self, id: BusId) {
        if let Some(b) = self.buses.get_mut(&id.0) {
            b.inserts.clear();
            b.effect_state.clear();
        }
    }

    pub fn insert_count(&self, id: BusId) -> usize {
        self.buses.get(&id.0).map(|b| b.inserts.len()).unwrap_or(0)
    }

    pub fn list_inserts(&self, id: BusId) -> Vec<InsertEffect> {
        self.buses
            .get(&id.0)
            .map(|b| b.inserts.clone())
            .unwrap_or_default()
    }

    pub fn set_insert(&mut self, id: BusId, index: usize, effect: InsertEffect) {
        if let Some(b) = self.buses.get_mut(&id.0) {
            if index < b.inserts.len() {
                b.inserts[index] = effect;
            }
        }
    }

    pub fn remove_insert(&mut self, id: BusId, index: usize) {
        if let Some(b) = self.buses.get_mut(&id.0) {
            if index < b.inserts.len() {
                b.inserts.remove(index);
                if index < b.effect_state.len() {
                    b.effect_state.remove(index);
                }
            }
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

    pub fn prepare(&mut self, frames: usize, channels: usize) {
        for b in self.buses.values_mut() {
            b.buffer.resize(frames * channels, 0.0);
            b.buffer.fill(0.0);
        }
    }

    pub fn accumulate_voice(&mut self, voice_id: u64, voice_out: &[f32]) {
        let bus_id = self.bus_for_voice(voice_id);
        if let Some(b) = self.buses.get_mut(&bus_id.0) {
            let n = b.buffer.len().min(voice_out.len());
            for i in 0..n {
                b.buffer[i] += voice_out[i];
            }
        }
    }

    pub fn finish(&mut self, out: &mut [f32], channels: usize, sample_rate: u32) {
        let any_solo = self.any_solo();
        let mut ids: Vec<u32> = self.buses.keys().copied().collect();
        ids.sort_unstable();

        for id in ids.iter().rev() {
            if *id == self.master_id.0 {
                continue;
            }
            let (parent, buf_copy, _silenced) = {
                let Some(bus) = self.buses.get_mut(id) else {
                    continue;
                };
                let silenced = any_solo && !bus.solo;
                if silenced {
                    bus.buffer.fill(0.0);
                }
                let vol = bus.volume_linear();
                for s in bus.buffer.iter_mut() {
                    *s *= vol;
                }
                let inserts = bus.inserts.clone();
                while bus.effect_state.len() < inserts.len() {
                    bus.effect_state.push(EffectState::default());
                }
                let sc_keys: Vec<(usize, String)> = inserts
                    .iter()
                    .enumerate()
                    .filter_map(|(i, fx)| match fx {
                        InsertEffect::SidechainCompressor { source_bus, .. } => {
                            Some((i, source_bus.clone()))
                        }
                        _ => None,
                    })
                    .collect();
                let key_bufs: Vec<(usize, Vec<f32>)> = sc_keys
                    .iter()
                    .filter_map(|(i, name)| {
                        let src = self.find_by_name(name)?;
                        let buf = self.buses.get(&src.0)?.buffer.clone();
                        Some((*i, buf))
                    })
                    .collect();
                for (idx, (fx, st)) in inserts.iter().zip(bus.effect_state.iter_mut()).enumerate() {
                    if let InsertEffect::SidechainCompressor {
                        threshold_db,
                        ratio,
                        makeup_db,
                        ..
                    } = fx
                    {
                        if let Some((_, key)) = key_bufs.iter().find(|(i, _)| *i == idx) {
                            process_sidechain(
                                &mut bus.buffer,
                                key,
                                *threshold_db,
                                *ratio,
                                *makeup_db,
                                st,
                                sample_rate,
                            );
                        }
                    } else {
                        fx.process(&mut bus.buffer, channels, sample_rate, st);
                    }
                }
                bus.meter.update(&bus.buffer);
                (bus.parent, bus.buffer.clone(), silenced)
            };
            if let Some(pid) = parent {
                if let Some(p) = self.buses.get_mut(&pid.0) {
                    let n = p.buffer.len().min(buf_copy.len());
                    for i in 0..n {
                        p.buffer[i] += buf_copy[i];
                    }
                }
            }
        }

        if let Some(master) = self.buses.get_mut(&self.master_id.0) {
            let vol = master.volume_linear();
            for s in master.buffer.iter_mut() {
                *s *= vol;
            }
            let inserts = master.inserts.clone();
            while master.effect_state.len() < inserts.len() {
                master.effect_state.push(EffectState::default());
            }
            let sc_keys: Vec<(usize, String)> = inserts
                .iter()
                .enumerate()
                .filter_map(|(i, fx)| match fx {
                    InsertEffect::SidechainCompressor { source_bus, .. } => {
                        Some((i, source_bus.clone()))
                    }
                    _ => None,
                })
                .collect();
            let key_bufs: Vec<(usize, Vec<f32>)> = sc_keys
                .iter()
                .filter_map(|(i, name)| {
                    let src = self.find_by_name(name)?;
                    let buf = self.buses.get(&src.0)?.buffer.clone();
                    Some((*i, buf))
                })
                .collect();
            for (idx, (fx, st)) in inserts.iter().zip(master.effect_state.iter_mut()).enumerate() {
                if let InsertEffect::SidechainCompressor {
                    threshold_db,
                    ratio,
                    makeup_db,
                    ..
                } = fx
                {
                    if let Some((_, key)) = key_bufs.iter().find(|(i, _)| *i == idx) {
                        process_sidechain(
                            &mut master.buffer,
                            key,
                            *threshold_db,
                            *ratio,
                            *makeup_db,
                            st,
                            sample_rate,
                        );
                    }
                } else {
                    fx.process(&mut master.buffer, channels, sample_rate, st);
                }
            }
            master.meter.update(&master.buffer);
            let n = out.len().min(master.buffer.len());
            out[..n].copy_from_slice(&master.buffer[..n]);
            if out.len() > n {
                out[n..].fill(0.0);
            }
        } else {
            out.fill(0.0);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub name: String,
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

#[derive(Debug, Default)]
pub struct SnapshotPlayer {
    base: HashMap<u32, f32>,
    active: Option<(String, f32)>,
    snaps: Vec<Snapshot>,
}

impl SnapshotPlayer {
    pub fn add(&mut self, snap: Snapshot) {
        self.snaps.push(snap);
    }

    pub fn capture_base(&mut self, mixer: &Mixer) {
        self.base.clear();
        for b in mixer.buses() {
            self.base.insert(b.id.0, b.volume_db);
        }
    }

    pub fn trigger(&mut self, name: &str, weight: f32) {
        self.active = Some((name.to_string(), weight.clamp(0.0, 1.0)));
    }

    pub fn clear(&mut self) {
        self.active = None;
    }

    pub fn names(&self) -> Vec<String> {
        self.snaps.iter().map(|s| s.name.clone()).collect()
    }

    pub fn active_name(&self) -> Option<&str> {
        self.active.as_ref().map(|(n, _)| n.as_str())
    }

    pub fn active_weight(&self) -> f32 {
        self.active.as_ref().map(|(_, w)| *w).unwrap_or(0.0)
    }

    pub fn base_volumes(&self) -> &HashMap<u32, f32> {
        &self.base
    }

    pub fn capture_named(&mut self, name: impl Into<String>, mixer: &Mixer) {
        let name = name.into();
        let mut snap = Snapshot::new(name.clone());
        for b in mixer.buses() {
            snap.set_volume(b.id, b.volume_db);
        }
        self.snaps.retain(|s| s.name != name);
        self.snaps.push(snap);
        if self.base.is_empty() {
            self.capture_base(mixer);
        }
    }

    pub fn apply(&self, mixer: &mut Mixer) {
        if let Some((ref name, w)) = self.active {
            if let Some(snap) = self.snaps.iter().find(|s| s.name == *name) {
                for (id, &target) in &snap.volumes {
                    let base = self.base.get(id).copied().unwrap_or(0.0);
                    let db = base + (target - base) * w;
                    mixer.set_volume_db(BusId(*id), db);
                }
            }
        }
    }
}
