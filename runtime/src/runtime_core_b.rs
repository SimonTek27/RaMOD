impl Runtime {
    pub fn render(&mut self, out: &mut [f32]) {
        let t0 = std::time::Instant::now();
        self.sync_event_spatial_gains();
        self.snapshots.apply(&mut self.mixer);
        let ch = self.channels as usize;
        if ch == 0 || out.len() % ch != 0 {
            out.fill(0.0);
            return;
        }
        let frames = out.len() / ch;
        self.mixer.prepare(frames, ch);
        {
            let mixer = &mut self.mixer;
            let rate = self.sample_rate;
            let buffers = &self.buffers;
            self.voices.mix_each(frames, ch, rate, buffers, |voice_id, slice| {
                mixer.accumulate_voice(voice_id, slice);
            });
        }
        self.mixer.finish(out, ch, self.sample_rate);
        let g = self.master.gain();
        if (g - 1.0).abs() > f32::EPSILON {
            for s in out.iter_mut() {
                *s *= g;
            }
        }
        self.last_render_us = t0.elapsed().as_micros().min(u128::from(u32::MAX)) as u32;
        self.spectrum.analyze(out, ch);
        self.capture_profiler_frame();
        if let Some(ref backend) = self.backend {
            backend.push_samples(out);
        }
    }

    pub fn profiler_enable(&mut self, on: bool) {
        self.profiler.enable(on);
    }
    pub fn profiler_clear(&mut self) {
        self.profiler.clear();
    }
    pub fn profiler_session(&self) -> &crate::profiler::ProfilerSession {
        &self.profiler
    }
    pub fn profiler_latest(&self) -> Option<&crate::profiler::ProfilerFrame> {
        self.profiler.latest()
    }
    pub fn spectrum(&self) -> &crate::profiler::SpectrumAnalyzer {
        &self.spectrum
    }
    pub fn last_render_cpu_us(&self) -> u32 {
        self.last_render_us
    }

    fn capture_profiler_frame(&mut self) {
        if !self.profiler.is_enabled() {
            return;
        }
        let master = self.mixer.master_id();
        let (peak, rms) = self
            .mixer
            .bus(master)
            .map(|b| (b.meter.peak, b.meter.rms))
            .unwrap_or((0.0, 0.0));
        let mut playing_events = Vec::new();
        let mut params = Vec::new();
        let mut emitters = Vec::new();
        for id in 1..1024u64 {
            let Some(inst) = self.events.instance(id) else {
                continue;
            };
            if !inst.is_playing() && !inst.started {
                continue;
            }
            let path = inst.path().as_str().to_string();
            playing_events.push(path.clone());
            if let Some(v) = inst.get_parameter("Intensity") {
                params.push((path.clone(), "Intensity".into(), v));
            }
            if inst.is_3d {
                let a = inst.attributes();
                let listener = self.events.listener();
                let d = crate::spatial::distance(&listener, &a);
                let aud = inst.attenuation.gain_at(d) * inst.gain_user;
                emitters.push(crate::profiler::EmitterDebug {
                    position: a.position,
                    velocity: a.velocity,
                    audibility: aud,
                });
            }
        }
        if playing_events.len() > 16 {
            playing_events.truncate(16);
        }
        self.profiler.push(crate::profiler::ProfilerFrame {
            index: 0,
            time: 0.0,
            cpu_render_us: self.last_render_us,
            active_voices: self.voices.active_count(),
            active_instances: self.events.instance_count(),
            master_peak: peak,
            master_rms: rms,
            playing_events,
            params,
            emitters,
        });
    }

    pub fn render_seconds(&mut self, seconds: f32) -> Vec<f32> {
        let frames = (seconds * self.sample_rate as f32).max(0.0) as usize;
        let mut out = vec![0.0f32; frames * self.channels as usize];
        self.render(&mut out);
        out
    }

    pub fn register_event(&mut self, desc: EventDescription) {
        self.events.register(desc);
    }

    pub fn register_designed_event(
        &mut self,
        event: DesignedEvent,
        buffers: &[BufferHandle],
    ) -> Result<(), RuntimeError> {
        let primary = event
            .tracks
            .iter()
            .flat_map(|t| t.instruments.iter())
            .find(|i| !i.buffer_slots.is_empty())
            .and_then(|i| i.buffer_slots.first().copied())
            .and_then(|slot| buffers.get(slot).copied())
            .ok_or(RuntimeError::InvalidBuffer)?;
        let mut desc = EventDescription::new(&event.path, primary)
            .with_priority(event.priority)
            .with_loop(
                event
                    .tracks
                    .iter()
                    .flat_map(|t| t.instruments.iter())
                    .any(|i| i.looping),
            )
            .with_output_bus(event.output_bus.clone());
        if event.is_3d {
            desc = desc.with_3d(event.attenuation);
        }
        self.events.register(desc);
        let handles: Vec<BufferHandle> = event
            .tracks
            .iter()
            .flat_map(|t| t.instruments.iter())
            .flat_map(|i| i.buffer_slots.iter().copied())
            .filter_map(|slot| buffers.get(slot).copied())
            .collect();
        self.designed_buffers.insert(event.path.clone(), handles);
        self.designed.push(event);
        Ok(())
    }

    /// Live event-instance count (also used by the Live Update server).
    pub fn instance_count(&self) -> usize {
        self.events.instance_count()
    }

    /// Start an event by path, wiring it to a voice and its output bus.
    pub fn play_event(&mut self, path: &str) -> Result<u64, RuntimeError> {
        let desc = self
            .events
            .get_description(path)
            .ok_or(RuntimeError::EventNotFound)?
            .clone();
        let bus = self
            .mixer
            .find_by_name(&desc.output_bus)
            .unwrap_or_else(|| self.mixer.default_bus());
        let buffer = desc.buffer;
        let params = PlayParams {
            gain: 1.0,
            looped: desc.looping,
        };
        let priority = desc.priority;
        let id = self
            .events
            .create_instance(path)
            .ok_or(RuntimeError::EventNotFound)?;
        let audibility = {
            let inst = self.events.instance(id).ok_or(RuntimeError::EventNotFound)?;
            let d = crate::spatial::distance(&self.events.listener(), &inst.attributes);
            inst.attenuation.gain_at(d) * inst.gain_user
        };
        self.current_output_bus = Some(bus);
        let played = self.play_with_priority(buffer, params, priority, audibility, None);
        self.current_output_bus = None;
        let handle = match played {
            Ok(h) => h,
            Err(e) => {
                self.events.release_instance(id);
                return Err(e);
            }
        };
        if let Some(inst) = self.events.instance_mut(id) {
            inst.voice = Some(handle);
            inst.started = true;
        }
        Ok(id)
    }

    /// Stop an instance and any voice attached to it.
    pub fn release_instance(&mut self, id: u64) {
        if let Some(inst) = self.events.release_instance(id) {
            if let Some(handle) = inst.voice {
                self.voices.stop(handle);
            }
        }
    }

    /// Move the 3D listener (camera / player).
    pub fn set_listener(&mut self, attributes: Attributes3d) {
        self.events.set_listener(attributes);
    }

    /// Move a 3D emitter; 3D gain/Doppler follow on the next render.
    pub fn set_instance_3d(&mut self, id: u64, attributes: Attributes3d) {
        if let Some(inst) = self.events.instance_mut(id) {
            inst.set_3d(attributes);
        }
    }

    /// Recompute per-voice gain and Doppler pitch from listener/emitter geometry.
    pub fn sync_event_spatial_gains(&mut self) {
        let listener = self.events.listener();
        let mut updates: Vec<(VoiceHandle, f32, f32)> = Vec::new();
        for inst in self.events.instances_mut() {
            let Some(handle) = inst.voice else { continue };
            if !inst.started {
                continue;
            }
            let (gain, pitch) = if inst.is_3d {
                let d = crate::spatial::distance(&listener, &inst.attributes);
                let cs = crate::spatial::closing_speed(&listener, &inst.attributes);
                (
                    inst.attenuation.gain_at(d) * inst.gain_user,
                    crate::spatial::doppler_pitch(cs),
                )
            } else {
                (inst.gain_user, 1.0)
            };
            updates.push((handle, gain, pitch));
        }
        for (handle, gain, pitch) in updates {
            self.voices.set_gain(handle, gain.max(0.0));
            self.voices.set_pitch(handle, pitch.clamp(0.25, 4.0));
        }
    }

    /// Set a named parameter on one live instance.
    pub fn set_instance_parameter(&mut self, id: u64, name: &str, value: f32) -> bool {
        self.events
            .instance_mut(id)
            .map(|i| i.set_parameter(name, value))
            .unwrap_or(false)
    }

    pub fn set_bus_volume_db(&mut self, id: BusId, db: f32) {
        self.mixer.set_volume_db(id, db);
    }

    pub fn set_bus_mute(&mut self, id: BusId, mute: bool) {
        self.mixer.set_mute(id, mute);
    }

    pub fn set_bus_solo(&mut self, id: BusId, solo: bool) {
        self.mixer.set_solo(id, solo);
    }

    /// Trigger a named snapshot, capturing current levels as its base.
    pub fn trigger_snapshot(&mut self, name: &str, weight: f32) {
        self.snapshots.capture_base(&self.mixer);
        self.snapshots.trigger(name, weight);
    }

    pub fn clear_snapshot(&mut self) {
        self.snapshots.clear();
    }
}

#[cfg(test)]
mod reconstruction_tests {
    use super::*;

    fn headless() -> Runtime {
        Runtime::new(RuntimeConfig {
            sample_rate: 48_000,
            channels: 2,
            max_voices: 8,
            open_device: false,
        })
        .expect("headless runtime")
    }

    #[test]
    fn play_event_creates_instance_and_voice() {
        let mut r = headless();
        let buf = r.load_pcm(vec![0.25; 4_800], 1, 48_000).expect("pcm");
        r.register_event(EventDescription::new("event:/sfx/test", buf));
        let id = r.play_event("event:/sfx/test").expect("play");
        assert_eq!(r.instance_count(), 1);
        assert_eq!(r.active_voices(), 1);
        assert!(r.set_instance_parameter(id, "Intensity", 0.5));
        r.release_instance(id);
        assert_eq!(r.instance_count(), 0);
    }

    #[test]
    fn unknown_event_reports_error_and_leaves_no_instance() {
        let mut r = headless();
        assert!(r.play_event("event:/nope").is_err());
        assert_eq!(r.instance_count(), 0);
    }

    #[test]
    fn render_emits_audible_samples_for_active_event() {
        let mut r = headless();
        let buf = r.load_pcm(vec![0.5; 4_800], 1, 48_000).expect("pcm");
        r.register_event(EventDescription::new("event:/sfx/loop", buf));
        r.play_event("event:/sfx/loop").expect("play");
        let out = r.render_seconds(0.01);
        assert_eq!(out.len(), 480 * 2);
        assert!(
            out.iter().any(|s| s.abs() > 0.0),
            "expected audible samples from the reconstructed render path"
        );
    }
}
