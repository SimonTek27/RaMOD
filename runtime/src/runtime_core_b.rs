
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
    pub fn create_child_bus(&mut self, parent: BusId, name: &str) -> BusId {
        self.mixer.add_bus(name, Some(parent))
    }
    pub fn set_bus_parent(&mut self, bus: BusId, parent: Option<BusId>) {
        if let Some(b) = self.mixer.bus_mut(bus) {
            b.parent = parent;
        }
    }
    pub fn set_bus_send_level(&mut self, bus: BusId, target_name: &str, level: f32) {
        self.mixer.set_send_level(bus, target_name, level);
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
                parent_id: b.parent.map(|p| p.0),
            })
            .collect()
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
        self.last_render_us
    }
    pub fn master_levels(&self) -> (f32, f32) {
        let m = self.mixer.master_id();
        self.mixer
            .bus(m)
            .map(|b| (b.meter.peak, b.meter.rms))
            .unwrap_or((0.0, 0.0))
    }
    pub fn spectrum(&self) -> &crate::profiler::SpectrumAnalyzer {
        &self.spectrum
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

    pub fn capture_snapshot_base(&mut self) {
        self.snapshots.capture_base(&self.mixer);
    }
    pub fn capture_named_snapshot(&mut self, name: &str) {
        self.snapshots.capture_named(name, &self.mixer);
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

    pub fn set_listener(&mut self, attrs: Attributes3d) {
        self.events.set_listener(attrs);
    }
    pub fn listener(&self) -> Attributes3d {
        self.events.listener()
    }
    pub fn set_instance_3d(&mut self, id: u64, attrs: Attributes3d) {
        if let Some(inst) = self.events.instance_mut(id) {
            inst.set_3d(attrs);
        }
    }
    pub fn set_instance_parameter(&mut self, id: u64, name: &str, value: f32) {
        if let Some(inst) = self.events.instance_mut(id) {
            inst.set_parameter(name, value);
        }
    }
    pub fn release_instance(&mut self, id: u64) {
        self.events.release(id, &mut self.voices);
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
        self.profiler.push(crate::profiler::ProfilerFrame {
            index: 0,
            time: 0.0,
            cpu_render_us: self.last_render_us,
            active_voices: self.voices.active_count(),
            active_instances: self.events.instance_count(),
            master_peak: peak,
            master_rms: rms,
            playing_events: Vec::new(),
            params: Vec::new(),
            emitters: Vec::new(),
        });
    }

    fn sync_event_spatial_gains(&mut self) {
        let _ = self;
    }

    pub fn tick(&mut self, dt: f32) {
        self.voices.advance_envelopes(dt);
        let _ = dt;
    }
}
