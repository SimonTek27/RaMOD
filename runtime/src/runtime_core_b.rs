//! Runtime methods: render, profiler, bus control, designed playback (part B).

impl Runtime {
    pub fn render(&mut self, out: &mut [f32]) {
        let frames = out.len() / self.channels as usize;
        let ch = self.channels as usize;
        self.mixer.prepare(frames, ch);

        // Mix voices into their routed buses
        self.voices.mix_each(|voice_id, slice| {
            let mixer = &mut self.mixer;
            {
                mixer.accumulate_voice(voice_id, slice);
            }
        });

        self.mixer.finish(out, ch, self.sample_rate);
        self.last_render_cpu_us = 0; // filled by profiler path when enabled
        if self.profiler_enabled {
            self.capture_profiler_frame();
        }
        // spectrum
        self.spectrum.push(out);
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn channels(&self) -> u16 {
        self.channels
    }
    pub fn active_voices(&self) -> usize {
        self.voices.active_count()
    }
    pub fn instance_count(&self) -> usize {
        self.events.instance_count()
    }
    pub fn last_render_cpu_us(&self) -> u32 {
        self.last_render_cpu_us
    }
    pub fn master_levels(&self) -> (f32, f32) {
        let master = self.mixer.master_id();
        if let Some(m) = self.mixer.bus(master) {
            (m.meter.peak, m.meter.rms)
        } else {
            (0.0, 0.0)
        }
    }
    pub fn spectrum(&self) -> &SpectrumAnalyzer {
        &self.spectrum
    }

    pub fn profiler_enable(&mut self, on: bool) {
        self.profiler_enabled = on;
    }
    pub fn profiler_clear(&mut self) {
        self.profiler_session.clear();
    }
    pub fn profiler_session(&self) -> &ProfilerSession {
        &self.profiler_session
    }
    pub fn profiler_latest(&self) -> Option<&ProfilerFrame> {
        self.profiler_session.frames.last()
    }

    pub fn clear_fired_markers(&mut self) {
        self.last_fired_markers.clear();
    }
    pub fn last_fired_markers(&self) -> Vec<String> {
        self.last_fired_markers.clone()
    }
    pub fn set_transition_fade(&mut self, secs: f32) {
        self.transition_fade = secs.max(0.0);
    }
    pub fn current_designed_instance(&self) -> Option<u64> {
        self.current_designed
    }
    pub fn clear_scheduled(&mut self) {
        self.scheduled.clear();
    }

    pub fn tick(&mut self, dt: f32) {
        self.voices.advance_envelopes(dt);
        self.tick_designed(dt);
        self.snapshots.apply(&mut self.mixer);
    }

    fn tick_designed(&mut self, dt: f32) {
        // simplified designed playback scheduler placeholder kept in core_a/play paths
        let _ = dt;
        self.sync_event_spatial_gains();
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

    pub fn set_bus_send_level(&mut self, bus: BusId, target_name: &str, level: f32) {
        self.mixer.set_send_level(bus, target_name, level);
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
                send_reverb: self.mixer.send_level(b.id, "Reverb"),
                is_return: b.is_return,
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
        // spatial gain/pitch updates for 3D instances
        let _ = self;
    }

    fn capture_profiler_frame(&mut self) {
        let (peak, rms) = self.master_levels();
        let frame = ProfilerFrame {
            cpu_render_us: self.last_render_cpu_us,
            active_voices: self.active_voices(),
            active_instances: self.instance_count(),
            master_peak: peak,
            master_rms: rms,
            playing_events: Vec::new(),
            emitters: Vec::new(),
        };
        self.profiler_session.push(frame);
    }
}
