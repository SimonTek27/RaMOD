//! RamodPlugin + systems + bank.json / FSB load + parameter sync + despawn cleanup + Live Update.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use ramod::{decode_sample_to_pcm, Fsb5, SoundFormat};
use ramod_runtime::{
    live_update::LiveUpdateServer, Attributes3d, Attenuation, EventDescription, Runtime,
    RuntimeConfig, Vec3 as RVec3,
};
use ramod_waveform::WaveBuffer;

use crate::assets::{BankEventAsset, RamodBankAsset, RamodBankLoader};
use crate::components::{
    AudioEmitter, AudioListener, AudioParameter, AudioParameters, LoadBank, PlayOnSpawn, Velocity,
};

/// Entity → live runtime instance id (for release on despawn).
#[derive(Resource, Default)]
pub struct InstanceMap {
    map: HashMap<Entity, u64>,
}

impl InstanceMap {
    fn insert(&mut self, entity: Entity, id: u64) {
        self.map.insert(entity, id);
    }

    fn remove(&mut self, entity: Entity) -> Option<u64> {
        self.map.remove(&entity)
    }
}

/// Optional Live Update TCP server (editor connects to game).
#[derive(Resource)]
pub struct LiveUpdateHub {
    pub server: Option<LiveUpdateServer>,
}

impl LiveUpdateHub {
    pub fn bind_default() -> Self {
        match LiveUpdateServer::bind_default() {
            Ok(server) => {
                bevy::log::info!("Live Update listening on {}", server.local_addr());
                Self {
                    server: Some(server),
                }
            }
            Err(e) => {
                bevy::log::warn!("Live Update bind failed: {e}");
                Self { server: None }
            }
        }
    }

    pub fn disabled() -> Self {
        Self { server: None }
    }
}

/// Global audio runtime resource.
#[derive(Resource)]
pub struct RamodAudio {
    pub runtime: Runtime,
    /// Event paths registered from banks (for debug).
    pub registered_events: Vec<String>,
    dt_accum: f32,
}

impl RamodAudio {
    pub fn new(open_device: bool) -> Result<Self, String> {
        let runtime = Runtime::new(RuntimeConfig {
            open_device,
            sample_rate: 48_000,
            channels: 2,
            max_voices: 64,
        })
        .map_err(|e| e.to_string())?;
        Ok(Self {
            runtime,
            registered_events: Vec::new(),
            dt_accum: 0.0,
        })
    }

    pub fn register_demo_tone(&mut self, path: &str, freq_hz: f32) {
        let rate = self.runtime.sample_rate();
        let frames = rate as usize / 2;
        let pcm: Vec<f32> = (0..frames)
            .map(|i| {
                let t = i as f32 / rate as f32;
                (2.0 * std::f32::consts::PI * freq_hz * t).sin() * 0.25
            })
            .collect();
        if let Ok(buf) = self.runtime.load_pcm(pcm, 1, rate) {
            self.runtime.register_event(
                EventDescription::new(path, buf).with_parameter("Intensity", 1.0),
            );
            if !self.registered_events.iter().any(|e| e == path) {
                self.registered_events.push(path.to_string());
            }
        }
    }

    pub fn load_bank_path(&mut self, path: &Path) -> Result<usize, String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "fsb" => self.load_fsb(path),
            "json" => self.load_bank_dir(path),
            _ => {
                if path.is_dir() {
                    let json = path.join("bank.json");
                    let fsb = path.join("Master.fsb");
                    if json.exists() {
                        return self.load_bank_dir(&json);
                    }
                    if fsb.exists() {
                        return self.load_fsb(&fsb);
                    }
                }
                Err(format!(
                    "unsupported bank path '{}' (want .fsb, bank.json, or bank dir)",
                    path.display()
                ))
            }
        }
    }

    pub fn load_bank_dir(&mut self, bank_json: &Path) -> Result<usize, String> {
        let bank = RamodBankAsset::from_json_path(bank_json).map_err(|e| e.to_string())?;
        let base = bank.base_dir.clone().unwrap_or_else(|| {
            bank_json
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf()
        });
        let fsb_pcm = load_fsb_pcm_index(&base, &bank.name);
        self.register_bank_asset_with_fsb(&bank, fsb_pcm.as_ref())
    }

    pub fn load_fsb(&mut self, path: &Path) -> Result<usize, String> {
        let bank =
            Fsb5::from_file(path).map_err(|e| format!("FSB open {}: {e}", path.display()))?;
        let mut count = 0usize;

        for i in 0..bank.len() {
            let Some(sample) = bank.sample(i) else {
                continue;
            };
            let event_path = format!("event:/fsb/{}", sample.name);

            let (samples, channels, sample_rate) = match fsb_sample_to_pcm(&bank, i) {
                Ok(v) => v,
                Err(e) => {
                    bevy::log::warn!("FSB sample '{}': {e}", sample.name);
                    continue;
                }
            };

            let buf = match self.runtime.load_pcm(samples, channels, sample_rate) {
                Ok(b) => b,
                Err(e) => {
                    bevy::log::warn!("load_pcm {}: {e}", sample.name);
                    continue;
                }
            };

            let mut desc =
                EventDescription::new(&event_path, buf).with_parameter("Intensity", 1.0);
            if sample.loop_start.is_some() {
                desc = desc.with_loop(true);
            }
            self.runtime.register_event(desc);

            if !self.registered_events.iter().any(|e| e == &event_path) {
                self.registered_events.push(event_path);
            }
            count += 1;
        }

        bevy::log::info!(
            "RamodAudio: registered {count}/{} samples from FSB '{}'",
            bank.len(),
            path.display()
        );
        Ok(count)
    }

    pub fn register_bank_asset(&mut self, bank: &RamodBankAsset) -> Result<usize, String> {
        self.register_bank_asset_with_fsb(bank, None)
    }

    pub fn register_bank_asset_with_fsb(
        &mut self,
        bank: &RamodBankAsset,
        fsb_pcm: Option<&HashMap<String, (Vec<f32>, u16, u32)>>,
    ) -> Result<usize, String> {
        let base = bank
            .base_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));
        let mut count = 0;
        let mut from_fsb = 0usize;
        for ev in &bank.events {
            match self.register_one_event(&base, ev, fsb_pcm) {
                Ok(used_fsb) => {
                    count += 1;
                    if used_fsb {
                        from_fsb += 1;
                    }
                    if !self.registered_events.iter().any(|e| e == &ev.path) {
                        self.registered_events.push(ev.path.clone());
                    }
                }
                Err(e) => {
                    bevy::log::warn!("bank event {}: {e}", ev.path);
                }
            }
        }
        bevy::log::info!(
            "RamodAudio: registered {count}/{} events from bank '{}' ({} from FSB)",
            bank.events.len(),
            bank.name,
            from_fsb
        );
        Ok(count)
    }

    fn register_one_event(
        &mut self,
        base: &Path,
        ev: &BankEventAsset,
        fsb_pcm: Option<&HashMap<String, (Vec<f32>, u16, u32)>>,
    ) -> Result<bool, String> {
        let (samples, channels, sample_rate, used_fsb) = resolve_pcm_hybrid(base, ev, fsb_pcm)?;
        let buf = self
            .runtime
            .load_pcm(samples, channels, sample_rate)
            .map_err(|e| e.to_string())?;

        let mut desc = EventDescription::new(&ev.path, buf)
            .with_loop(ev.looping)
            .with_priority(ev.priority);

        if ev.is_3d {
            desc = desc.with_3d(Attenuation {
                min_distance: ev.min_distance,
                max_distance: ev.max_distance,
            });
        }
        for (name, default) in &ev.parameters {
            desc = desc.with_parameter(name.clone(), *default);
        }
        if !ev.parameters.iter().any(|(n, _)| n == "Intensity") {
            desc = desc.with_parameter("Intensity", 1.0);
        }

        self.runtime.register_event(desc);
        Ok(used_fsb)
    }

    pub fn play_event(&mut self, path: &str) -> Option<u64> {
        self.runtime.play_event(path).ok()
    }

    pub fn stop_instance(&mut self, id: u64) {
        self.runtime.release_instance(id);
    }
}

fn fsb_sample_to_pcm(bank: &Fsb5, index: usize) -> Result<(Vec<f32>, u16, u32), String> {
    let sample = bank
        .sample(index)
        .ok_or_else(|| format!("sample {index} missing"))?;

    if matches!(bank.format, SoundFormat::Vorbis) {
        let decoded =
            decode_sample_to_pcm(bank, index).map_err(|e| format!("vorbis decode: {e}"))?;
        let ch = decoded.channels.max(1) as u16;
        let rate = if decoded.sample_rate == 0 {
            48_000
        } else {
            decoded.sample_rate
        };
        return Ok((decoded.samples, ch, rate));
    }

    let data = bank
        .sample_data(index)
        .ok_or_else(|| format!("sample {index} data missing"))?;

    let channels = sample.channels.max(1) as u16;
    let rate = if sample.frequency == 0 {
        48_000
    } else {
        sample.frequency
    };

    let samples = match bank.format {
        SoundFormat::Pcm8 => data.iter().map(|&b| (b as i8 as f32) / 128.0).collect(),
        SoundFormat::Pcm16 => data
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
            .collect(),
        SoundFormat::Pcm32 => data
            .chunks_exact(4)
            .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2147483648.0)
            .collect(),
        SoundFormat::PcmFloat => data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
        other => {
            return Err(format!(
                "format {other} not supported for FSB load (PCM or Vorbis only)"
            ));
        }
    };

    Ok((samples, channels, rate))
}

fn resolve_pcm_hybrid(
    base: &Path,
    ev: &BankEventAsset,
    fsb_pcm: Option<&HashMap<String, (Vec<f32>, u16, u32)>>,
) -> Result<(Vec<f32>, u16, u32, bool), String> {
    if let Some(map) = fsb_pcm {
        for key in event_fsb_keys(ev) {
            if let Some((samples, ch, rate)) = map.get(&key) {
                return Ok((samples.clone(), *ch, *rate, true));
            }
        }
    }

    if let Some(ref rel) = ev.asset_file {
        let full = base.join(rel);
        if full.exists() {
            let w = WaveBuffer::from_file(&full).map_err(|e| format!("{}: {e}", full.display()))?;
            return Ok((w.samples, w.channels, w.sample_rate, false));
        }
        return Err(format!("asset file not found: {}", full.display()));
    }

    let rate = 48_000u32;
    let frames = rate as usize / 4;
    let freq = 440.0;
    let samples: Vec<f32> = (0..frames)
        .map(|i| {
            let t = i as f32 / rate as f32;
            (2.0 * std::f32::consts::PI * freq * t).sin() * 0.2
        })
        .collect();
    Ok((samples, 1, rate, false))
}

fn event_fsb_keys(ev: &BankEventAsset) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(ref a) = ev.asset {
        keys.push(a.clone());
    }
    if let Some(ref f) = ev.asset_file {
        let p = Path::new(f);
        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
            keys.push(stem.to_string());
        }
        if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
            keys.push(name.to_string());
        }
    }
    if let Some(tail) = ev.path.rsplit('/').next() {
        if !tail.is_empty() {
            keys.push(tail.to_string());
        }
    }
    keys
}

fn load_fsb_pcm_index(
    base: &Path,
    bank_name: &str,
) -> Option<HashMap<String, (Vec<f32>, u16, u32)>> {
    let candidates = [
        base.join(format!("{bank_name}.fsb")),
        base.join("Master.fsb"),
    ];
    let path = candidates.into_iter().find(|p| p.exists())?;
    let bank = match Fsb5::from_file(&path) {
        Ok(b) => b,
        Err(e) => {
            bevy::log::warn!("hybrid FSB {}: {e}", path.display());
            return None;
        }
    };
    let mut map = HashMap::new();
    for i in 0..bank.len() {
        let Some(sample) = bank.sample(i) else {
            continue;
        };
        match fsb_sample_to_pcm(&bank, i) {
            Ok(pcm) => {
                map.insert(sample.name.clone(), pcm);
            }
            Err(e) => bevy::log::warn!("FSB sample '{}': {e}", sample.name),
        }
    }
    bevy::log::info!(
        "hybrid: indexed {} samples from {}",
        map.len(),
        path.display()
    );
    Some(map)
}

pub struct RamodPlugin {
    pub open_device: bool,
    /// Bind TCP Live Update server (default port 17420).
    pub enable_live_update: bool,
}

impl Default for RamodPlugin {
    fn default() -> Self {
        Self {
            open_device: true,
            enable_live_update: true,
        }
    }
}

impl Plugin for RamodPlugin {
    fn build(&self, app: &mut App) {
        let audio = RamodAudio::new(self.open_device).unwrap_or_else(|e| {
            bevy::log::error!("RamodAudio init failed: {e}; falling back to headless");
            RamodAudio::new(false).expect("headless runtime")
        });

        let live = if self.enable_live_update {
            LiveUpdateHub::bind_default()
        } else {
            LiveUpdateHub::disabled()
        };

        app.insert_resource(audio)
            .insert_resource(live)
            .init_resource::<InstanceMap>()
            .init_asset::<RamodBankAsset>()
            .register_asset_loader(RamodBankLoader)
            .add_systems(
                PostUpdate,
                (
                    process_load_bank,
                    sync_listener,
                    start_emitters,
                    sync_emitter_volume,
                    sync_audio_parameters,
                    sync_emitters_3d,
                    cleanup_despawned_emitters,
                    live_update_poll,
                    audio_tick_render,
                )
                    .chain(),
            );
    }
}

fn process_load_bank(
    mut commands: Commands,
    mut audio: ResMut<RamodAudio>,
    q: Query<(Entity, &LoadBank), Added<LoadBank>>,
) {
    for (entity, load) in &q {
        match audio.load_bank_path(&load.path) {
            Ok(n) => bevy::log::info!("LoadBank {}: {n} events/samples", load.path.display()),
            Err(e) => bevy::log::error!("LoadBank {}: {e}", load.path.display()),
        }
        commands.entity(entity).remove::<LoadBank>();
    }
}

fn sync_listener(
    mut audio: ResMut<RamodAudio>,
    q: Query<(&GlobalTransform, Option<&Velocity>), With<AudioListener>>,
) {
    let Ok((gt, vel)) = q.single() else {
        return;
    };
    let t = gt.translation();
    let velocity = vel.map(|v| v.0).unwrap_or(Vec3::ZERO);
    audio.runtime.set_listener(Attributes3d {
        position: RVec3 {
            x: t.x,
            y: t.y,
            z: t.z,
        },
        velocity: RVec3 {
            x: velocity.x,
            y: velocity.y,
            z: velocity.z,
        },
        ..Default::default()
    });
}

fn start_emitters(
    mut commands: Commands,
    mut audio: ResMut<RamodAudio>,
    mut instances: ResMut<InstanceMap>,
    mut q: Query<
        (
            Entity,
            &mut AudioEmitter,
            &GlobalTransform,
            Option<&AudioParameter>,
            Option<&AudioParameters>,
        ),
        Added<PlayOnSpawn>,
    >,
) {
    for (entity, mut emitter, gt, one_param, multi) in &mut q {
        if emitter.instance_id.is_some() {
            commands.entity(entity).remove::<PlayOnSpawn>();
            continue;
        }
        match audio.runtime.play_event(&emitter.event) {
            Ok(id) => {
                if emitter.is_3d {
                    let t = gt.translation();
                    audio.runtime.set_instance_3d(
                        id,
                        Attributes3d {
                            position: RVec3 {
                                x: t.x,
                                y: t.y,
                                z: t.z,
                            },
                            ..Default::default()
                        },
                    );
                }
                let _ = audio
                    .runtime
                    .set_instance_parameter(id, "Intensity", emitter.volume);
                if let Some(p) = one_param {
                    let _ = audio
                        .runtime
                        .set_instance_parameter(id, &p.name, p.value);
                }
                if let Some(ps) = multi {
                    for (name, value) in &ps.0 {
                        let _ = audio.runtime.set_instance_parameter(id, name, *value);
                    }
                }
                emitter.instance_id = Some(id);
                instances.insert(entity, id);
            }
            Err(e) => {
                bevy::log::warn!("play_event {}: {e}", emitter.event);
            }
        }
        commands.entity(entity).remove::<PlayOnSpawn>();
    }
}

fn sync_emitter_volume(mut audio: ResMut<RamodAudio>, q: Query<&AudioEmitter>) {
    for emitter in &q {
        let Some(id) = emitter.instance_id else {
            continue;
        };
        let _ = audio
            .runtime
            .set_instance_parameter(id, "Intensity", emitter.volume);
    }
}

fn sync_audio_parameters(
    mut audio: ResMut<RamodAudio>,
    q: Query<(
        &AudioEmitter,
        Option<&AudioParameter>,
        Option<&AudioParameters>,
    )>,
) {
    for (emitter, one, multi) in &q {
        let Some(id) = emitter.instance_id else {
            continue;
        };
        if let Some(p) = one {
            let _ = audio
                .runtime
                .set_instance_parameter(id, &p.name, p.value);
        }
        if let Some(ps) = multi {
            for (name, value) in &ps.0 {
                let _ = audio.runtime.set_instance_parameter(id, name, *value);
            }
        }
    }
}

fn sync_emitters_3d(
    mut audio: ResMut<RamodAudio>,
    q: Query<(&AudioEmitter, &GlobalTransform, Option<&Velocity>)>,
) {
    for (emitter, gt, vel) in &q {
        let Some(id) = emitter.instance_id else {
            continue;
        };
        if !emitter.is_3d {
            continue;
        }
        let t = gt.translation();
        let velocity = vel.map(|v| v.0).unwrap_or(Vec3::ZERO);
        audio.runtime.set_instance_3d(
            id,
            Attributes3d {
                position: RVec3 {
                    x: t.x,
                    y: t.y,
                    z: t.z,
                },
                velocity: RVec3 {
                    x: velocity.x,
                    y: velocity.y,
                    z: velocity.z,
                },
                ..Default::default()
            },
        );
    }
}

fn cleanup_despawned_emitters(
    mut audio: ResMut<RamodAudio>,
    mut instances: ResMut<InstanceMap>,
    mut removed: RemovedComponents<AudioEmitter>,
) {
    for entity in removed.read() {
        if let Some(id) = instances.remove(entity) {
            audio.runtime.release_instance(id);
            bevy::log::debug!("released instance {id} for despawned {entity:?}");
        }
    }
}

fn live_update_poll(mut audio: ResMut<RamodAudio>, mut live: ResMut<LiveUpdateHub>) {
    if let Some(server) = live.server.as_mut() {
        if let Err(e) = server.poll(&mut audio.runtime) {
            bevy::log::debug!("live poll: {e}");
        }
    }
}

fn audio_tick_render(time: Res<Time>, mut audio: ResMut<RamodAudio>) {
    let dt = time.delta_secs();
    audio.runtime.tick(dt);
    let frames = ((dt * audio.runtime.sample_rate() as f32) as usize).clamp(64, 4096);
    let ch = audio.runtime.channels() as usize;
    let mut out = vec![0.0f32; frames * ch.max(1)];
    audio.runtime.render(&mut out);
}
