
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
        for (n, v) in &event.parameters {
            desc = desc.with_parameter(n.clone(), *v);
        }
        self.events.register(desc);
        self.designed_buffers
            .insert(event.path.clone(), buffers.to_vec());
        self.designed.push(event);
        Ok(())
    }

    pub fn play_designed(
        &mut self,
        path: &str,
        buffers: &[BufferHandle],
    ) -> Result<Vec<VoiceHandle>, RuntimeError> {
        let event = self
            .designed
            .iter()
            .find(|e| e.path == path)
            .cloned()
            .ok_or(RuntimeError::EventNotFound)?;
        let any_solo = event.tracks.iter().any(|t| t.solo);
        let mut handles = Vec::new();
        for track in &event.tracks {
            if track.mute {
                continue;
            }
            if any_solo && !track.solo {
                continue;
            }
            for inst in &track.instruments {
                let roll = simple_hash(&format!("{}:{}", path, inst.name)) as f32 / u64::MAX as f32;
                if roll > inst.probability.clamp(0.0, 1.0) {
                    continue;
                }
                let count = match inst.kind {
                    designer::InstrumentKind::Scatter => inst.scatter_count.max(1),
                    _ => 1,
                };
                for n in 0..count {
                    if inst.buffer_slots.is_empty() {
                        continue;
                    }
                    let slot = if inst.kind == designer::InstrumentKind::Multi {
                        let idx = (simple_hash(&(inst.name.clone() + &n.to_string())) as usize)
                            % inst.buffer_slots.len();
                        inst.buffer_slots[idx]
                    } else {
                        inst.buffer_slots[0]
                    };
                    let Some(&buf) = buffers.get(slot) else {
                        continue;
                    };
                    let params = PlayParams {
                        gain: 1.0,
                        looped: inst.looping,
                    };
                    if inst.start_time <= 0.001 {
                        match self.play_with_priority(
                            buf,
                            params,
                            event.priority,
                            1.0,
                            Some(inst.envelope),
                        ) {
                            Ok(h) => handles.push(h),
                            Err(_) => break,
                        }
                    } else {
                        self.scheduled.push(ScheduledVoice {
                            remaining: inst.start_time,
                            buffer: buf,
                            params,
                            priority: event.priority,
                            envelope: Some(inst.envelope),
                            instance_id: None,
                        });
                    }
                }
            }
        }
        Ok(handles)
    }

    pub fn play_designed_registered(
        &mut self,
        path: &str,
    ) -> Result<Vec<VoiceHandle>, RuntimeError> {
        let buffers = self
            .designed_buffers
            .get(path)
            .cloned()
            .ok_or(RuntimeError::EventNotFound)?;
        self.play_designed(path, &buffers)
    }

    pub fn designed_events(&self) -> &[DesignedEvent] {
        &self.designed
    }
    pub fn export_bank_manifest(&self, name: &str) -> BankManifest {
        BankManifest::from_events(name, &self.designed)
    }

    pub fn load_bank(&mut self, bank: Bank) {
        for spec in bank.events.iter().cloned() {
            self.events.register(spec.into_description());
        }
        self.banks.insert(bank);
    }

    pub fn set_listener(&mut self, attributes: Attr3d) {
        self.events.set_listener(attributes);
    }
    pub fn listener(&self) -> Attr3d {
        self.events.listener()
    }

    pub fn create_instance(&mut self, path: &str) -> Result<EventInstance, RuntimeError> {
        let id = self
            .events
            .create_instance(path)
            .ok_or(RuntimeError::EventNotFound)?;
        self.events
            .release_instance(id)
            .ok_or(RuntimeError::InvalidHandle)
    }

    pub fn start_instance(&mut self, mut inst: EventInstance) -> Result<u64, RuntimeError> {
        let audibility = self.audibility_for(&inst);
        let params = inst.play_params();
        let voice = self.play_with_priority(inst.buffer, params, inst.priority, audibility, None)?;
        inst.voice = Some(voice);
        inst.started = true;
        let id = inst.id;
        if self.events.instance(id).is_none() {
            self.events.instances_reinsert(inst);
        } else if let Some(slot) = self.events.instance_mut(id) {
            *slot = inst;
        }
        Ok(id)
    }

    pub fn play_event(&mut self, path: &str) -> Result<u64, RuntimeError> {
        if self.designed.iter().any(|e| e.path == path) {
            if let Some(ev) = self.designed.iter().find(|e| e.path == path) {
                self.current_output_bus = self.mixer.find_by_name(&ev.output_bus);
            }
            let mut handles = self.play_designed_registered(path)?;
            let mut inst = self.create_instance(path)?;
            inst.started = true;
            let id = inst.id;
            for s in self.scheduled.iter_mut() {
                if s.instance_id.is_none() {
                    s.instance_id = Some(id);
                }
            }
            self.designed_playbacks.push(DesignedPlayback {
                instance_id: id,
                path: path.to_string(),
                elapsed: 0.0,
                voices: std::mem::take(&mut handles),
                markers_fired: Vec::new(),
            });
            self.events.instances_reinsert(inst);
            return Ok(id);
        }
        if let Some(desc) = self.events.get_description(path) {
            self.current_output_bus = self.mixer.find_by_name(&desc.output_bus);
        }
        let inst = self.create_instance(path)?;
        self.start_instance(inst)
    }

    pub fn stop_instance(&mut self, id: u64) {
        if let Some(inst) = self.events.instance_mut(id) {
            if let Some(v) = inst.voice.take() {
                self.voices.stop(v);
            }
            inst.started = false;
        }
        if let Some(pos) = self
            .designed_playbacks
            .iter()
            .position(|p| p.instance_id == id)
        {
            let pb = self.designed_playbacks.remove(pos);
            for v in pb.voices {
                self.voices.stop(v);
            }
        }
        self.scheduled.retain(|s| s.instance_id != Some(id));
    }

    pub fn clear_scheduled(&mut self) {
        self.scheduled.clear();
    }

    pub fn master_levels(&self) -> (f32, f32) {
        let id = self.mixer.master_id();
        self.bus_meter(id)
            .map(|m| (m.peak, m.rms))
            .unwrap_or((0.0, 0.0))
    }

    pub fn last_fired_markers(&self) -> Vec<String> {
        self.recent_markers.clone()
    }

    pub fn clear_fired_markers(&mut self) {
        self.recent_markers.clear();
    }

    pub fn set_transition_fade(&mut self, secs: f32) {
        self.transition_fade_secs = secs.max(0.0);
    }

    pub fn transition_fade(&self) -> f32 {
        self.transition_fade_secs
    }

    pub fn current_designed_instance(&self) -> Option<u64> {
        self.designed_playbacks.last().map(|p| p.instance_id)
    }

    pub fn release_instance(&mut self, id: u64) {
        self.stop_instance(id);
        self.events.release_instance(id);
    }

    pub fn set_instance_parameter(&mut self, id: u64, name: &str, value: f32) -> bool {
        self.events
            .instance_mut(id)
            .map(|i| i.set_parameter(name, value))
            .unwrap_or(false)
    }

    pub fn set_instance_3d(&mut self, id: u64, attributes: Attr3d) {
        if let Some(i) = self.events.instance_mut(id) {
            i.set_3d(attributes);
        }
    }

    pub fn instance_count(&self) -> usize {
        self.events.instance_count()
    }

    pub fn set_bus_volume_db(&mut self, bus: BusId, db: f32) {
        self.mixer.set_volume_db(bus, db);
    }
    pub fn set_bus_mute(&mut self, bus: BusId, mute: bool) {
        self.mixer.set_mute(bus, mute);
    }
    pub fn set_bus_solo(&mut self, bus: BusId, solo: bool) {
        self.mixer.set_solo(bus, solo);
    }
    pub fn add_bus_insert(&mut self, bus: BusId, effect: InsertEffect) {
        self.mixer.add_insert(bus, effect);
    }

    pub fn clear_bus_inserts(&mut self, bus: BusId) {
        self.mixer.clear_inserts(bus);
    }

    pub fn bus_insert_count(&self, bus: BusId) -> usize {
        self.mixer.insert_count(bus)
    }

    pub fn list_bus_inserts(&self, bus: BusId) -> Vec<InsertEffect> {
        self.mixer.list_inserts(bus)
    }

    pub fn set_bus_insert(&mut self, bus: BusId, index: usize, effect: InsertEffect) {
        self.mixer.set_insert(bus, index, effect);
    }

    pub fn remove_bus_insert(&mut self, bus: BusId, index: usize) {
        self.mixer.remove_insert(bus, index);
    }

    pub fn register_snapshot(&mut self, snapshot: Snapshot) {
        self.snapshots.add(snapshot);
    }
    pub fn capture_snapshot_base(&mut self) {
        self.snapshots.capture_base(&self.mixer);
    }
    pub fn trigger_snapshot(&mut self, name: &str, weight: f32) {
        self.snapshots.trigger(name, weight);
        self.snapshots.apply(&mut self.mixer);
    }
    pub fn clear_snapshot(&mut self) {
        self.snapshots.clear();
        for (id, &db) in self.snapshots.base_volumes() {
            self.mixer.set_volume_db(BusId(id), db);
        }
    }

    pub fn snapshot_names(&self) -> Vec<String> {
        self.snapshots.names()
    }

    pub fn active_snapshot(&self) -> Option<(String, f32)> {
        self.snapshots
            .active_name()
            .map(|n| (n.to_string(), self.snapshots.active_weight()))
    }

    pub fn capture_named_snapshot(&mut self, name: &str) {
        self.snapshots.capture_named(name, &self.mixer);
    }

    pub fn bus_meter(&self, bus: BusId) -> Option<Meter> {
        self.mixer.bus(bus).map(|b| b.meter)
    }

    pub fn list_bus_meters(&self) -> Vec<BusMeterInfo> {
        self.mixer
            .buses()
            .map(|b| BusMeterInfo {
                id: b.id.0,
                name: b.name.clone(),
                peak: b.meter.peak,
                rms: b.meter.rms,
                mute: b.mute,
                solo: b.solo,
                volume_db: b.volume_db,
            })
            .collect()
    }

    fn audibility_for(&self, inst: &EventInstance) -> f32 {
        if !inst.is_3d {
            return 1.0;
        }
        let listener = self.events.listener();
        let d = distance(&listener, &inst.attributes);
        inst.attenuation.gain_at(d) * inst.gain_user
    }

    fn sync_event_spatial_gains(&mut self) {
        let listener = self.events.listener();
        let mut updates: Vec<(VoiceHandle, f32, f32)> = Vec::new();

        // Single-voice event instances
        for inst in self.events.instances_mut() {
            let Some(voice) = inst.voice else {
                continue;
            };
            let mut gain = inst.gain_user;
            let mut pitch = 1.0;
            if inst.is_3d {
                let d = distance(&listener, &inst.attributes);
                gain *= inst.attenuation.gain_at(d);
                let closing = closing_speed(&listener, &inst.attributes);
                pitch = spatial::doppler_pitch(closing);
            }
            if let Some(intensity) = inst.get_parameter("Intensity") {
                gain *= intensity.clamp(0.0, 4.0);
            }
            updates.push((voice, gain, pitch));
        }

        // Multi-track designed: same 3D/Doppler on every instrument voice
        let designed_pairs: Vec<(u64, Vec<VoiceHandle>)> = self
            .designed_playbacks
            .iter()
            .map(|pb| (pb.instance_id, pb.voices.clone()))
            .collect();
        for (instance_id, voices) in designed_pairs {
            let Some(inst) = self.events.instance(instance_id) else {
                continue;
            };
            let mut gain = inst.gain_user;
            let mut pitch = 1.0;
            if inst.is_3d {
                let d = distance(&listener, &inst.attributes);
                gain *= inst.attenuation.gain_at(d);
                let closing = closing_speed(&listener, &inst.attributes);
                pitch = spatial::doppler_pitch(closing);
            }
            if let Some(intensity) = inst.get_parameter("Intensity") {
                gain *= intensity.clamp(0.0, 4.0);
            }
            for voice in voices {
                if inst.voice == Some(voice) {
                    continue;
                }
                updates.push((voice, gain, pitch));
            }
        }

        for (voice, gain, pitch) in updates {
            self.voices.set_gain(voice, gain);
            self.voices.set_pitch(voice, pitch);
        }
    }
}

fn simple_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
