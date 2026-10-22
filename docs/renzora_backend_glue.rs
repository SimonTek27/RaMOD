//! Drop-in glue: `RamodRenzoraBackend` as Renzora's `audio_backend::Backend`.
//!
//! **Where to put this**
//!
//! Copy into a Renzora game/plugin crate that depends on:
//!
//! ```toml
//! [dependencies]
//! bevy_ramod = { git = "https://github.com/SimonTek27/RaMOD", features = ["renzora"] }
//! # `renzora` is already provided by the engine workspace
//! ```
//!
//! Then:
//!
//! ```ignore
//! app.add_audio_backend(RamodBackend::new(true).expect("ramod"));
//! // Do not register the default Kira backend.
//! ```
//!
//! This file is the zero-glue reference implementation of Option B in
//! `docs/RENZORA.md`.

use bevy_ramod::renzora::backend::{self as ramod_be, RamodRenzoraBackend};
use renzora::audio_backend::{
    AppAudioBackendExt, Backend, BackendInfo, BusState, Caps, CaptureInfo, ClipInfo, DeviceList,
    EmitterState, ListenerState, PlayRequest, StopRequest, StopTarget, UpdateReply, UpdateRequest,
};

/// Newtype so we can implement Renzora's `Backend` on RaMOD.
pub struct RamodBackend(pub RamodRenzoraBackend);

impl RamodBackend {
    pub fn new(open_device: bool) -> Result<Self, String> {
        Ok(Self(RamodRenzoraBackend::new(open_device)?))
    }

    pub fn inner(&self) -> &RamodRenzoraBackend {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut RamodRenzoraBackend {
        &mut self.0
    }
}

impl Backend for RamodBackend {
    fn name(&self) -> &str {
        self.0.name()
    }

    fn init(&mut self) -> Result<BackendInfo, String> {
        let i = self.0.init()?;
        Ok(BackendInfo {
            sample_rate: i.sample_rate,
            caps: Caps(i.caps.0),
            device: i.device,
        })
    }

    fn shutdown(&mut self) {
        self.0.shutdown();
    }

    fn set_buses(&mut self, buses: &[BusState]) {
        let mapped: Vec<ramod_be::BusState> = buses
            .iter()
            .map(|b| ramod_be::BusState {
                key: b.key.clone(),
                gain: b.gain,
                pan: b.pan,
                muted: b.muted,
                soloed: b.soloed,
                output: b.output.clone(),
                reverb_send: b.reverb_send,
                delay_send: b.delay_send,
            })
            .collect();
        self.0.set_buses(&mapped);
    }

    fn load_clip(&mut self, clip: u64, extension: &str, bytes: &[u8]) -> Result<ClipInfo, String> {
        let c = self.0.load_clip(clip, extension, bytes)?;
        Ok(ClipInfo {
            duration: c.duration,
            sample_rate: c.sample_rate,
        })
    }

    fn unload_clip(&mut self, clip: u64) {
        self.0.unload_clip(clip);
    }

    fn play(&mut self, request: &PlayRequest) -> Result<(), String> {
        let mapped = ramod_be::PlayRequest {
            voice: request.voice,
            clip: request.clip,
            bus: request.bus.clone(),
            gain: request.gain,
            pan: request.pan,
            pitch: request.pitch,
            looping: request.looping,
            fade_in: request.fade_in,
            start: request.start,
            emitter: request.emitter.map(|e| ramod_be::EmitterState {
                position: e.position,
                min_distance: e.min_distance,
                max_distance: e.max_distance,
                rolloff: e.rolloff,
            }),
            reverb_send: request.reverb_send,
            delay_send: request.delay_send,
        };
        self.0.play(&mapped)
    }

    fn stop(&mut self, request: &StopRequest) {
        let target = match &request.target {
            StopTarget::Voice(v) => ramod_be::StopTarget::Voice(*v),
            StopTarget::Bus(k) => ramod_be::StopTarget::Bus(k.clone()),
            StopTarget::All => ramod_be::StopTarget::All,
        };
        self.0.stop(&ramod_be::StopRequest {
            target,
            fade: request.fade,
        });
    }

    fn update(&mut self, request: &UpdateRequest) -> UpdateReply {
        let mapped = ramod_be::UpdateRequest {
            listener: request.listener.map(|l| ramod_be::ListenerState {
                position: l.position,
                right: l.right,
            }),
            moved: request.moved.clone(),
            gains: request.gains.clone(),
            pitches: request.pitches.clone(),
            buses: request.buses.clone(),
            pans: request.pans.clone(),
            emitters: request
                .emitters
                .iter()
                .map(|(v, e)| {
                    (
                        *v,
                        ramod_be::EmitterState {
                            position: e.position,
                            min_distance: e.min_distance,
                            max_distance: e.max_distance,
                            rolloff: e.rolloff,
                        },
                    )
                })
                .collect(),
            paused: request.paused.clone(),
        };
        let reply = self.0.update(&mapped);
        UpdateReply {
            peaks: reply.peaks,
            finished: reply.finished,
        }
    }

    fn open_capture(&mut self, capture: u64, device: Option<&str>) -> Result<CaptureInfo, String> {
        let _ = (capture, device);
        Err("ramod backend does not capture".into())
    }

    fn close_capture(&mut self, capture: u64) {
        self.0.close_capture(capture);
    }

    fn read_capture(&mut self, capture: u64) -> Vec<f32> {
        self.0.read_capture(capture)
    }

    fn push_frames(&mut self, bus: &str, samples: &[f32]) {
        self.0.push_frames(bus, samples);
    }

    fn list_devices(&mut self) -> DeviceList {
        let d = self.0.list_devices();
        DeviceList {
            inputs: d.inputs,
            outputs: d.outputs,
        }
    }
}

/// Helper: register RaMOD as the only Renzora audio backend.
pub trait AddRamodBackendExt {
    fn add_ramod_audio_backend(&mut self, open_device: bool) -> Result<&mut Self, String>;
}

impl AddRamodBackendExt for bevy::app::App {
    fn add_ramod_audio_backend(&mut self, open_device: bool) -> Result<&mut Self, String> {
        let backend = RamodBackend::new(open_device)?;
        self.add_audio_backend(backend);
        Ok(self)
    }
}
