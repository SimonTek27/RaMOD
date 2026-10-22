
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
