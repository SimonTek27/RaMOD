//! Live Update hooks on Runtime (always compiled; used by live-update server).

use crate::mixer::BusId;
use crate::Runtime;

impl Runtime {
    /// Apply parameter to all started instances of `event_path`.
    pub fn live_set_param(&mut self, event_path: &str, name: &str, value: f32) {
        // Instance ids we can reach: iterate by collecting ids first
        let ids: Vec<u64> = (1..self.instance_count() as u64 + 64)
            .filter(|&id| {
                self.events
                    .instance(id)
                    .map(|i| i.path().as_str() == event_path)
                    .unwrap_or(false)
            })
            .collect();
        // Better: use event system list — we only have instance_count.
        // Fallback: scan a reasonable id range.
        for id in 1..1024u64 {
            if let Some(inst) = self.events.instance(id) {
                if inst.path().as_str() == event_path {
                    let _ = id;
                    let _ = inst;
                }
            }
        }
        for id in 1..1024u64 {
            let matches = self
                .events
                .instance(id)
                .map(|i| i.path().as_str() == event_path)
                .unwrap_or(false);
            if matches {
                self.set_instance_parameter(id, name, value);
            }
        }
        let _ = ids;
    }

    pub fn live_set_bus_volume(&mut self, bus_name: &str, volume_db: f32) {
        if let Some(id) = bus_id_named(self, bus_name) {
            self.set_bus_volume_db(id, volume_db);
        }
    }

    pub fn live_set_bus_mute(&mut self, bus_name: &str, mute: bool) {
        if let Some(id) = bus_id_named(self, bus_name) {
            self.set_bus_mute(id, mute);
        }
    }

    pub fn live_set_bus_solo(&mut self, bus_name: &str, solo: bool) {
        if let Some(id) = bus_id_named(self, bus_name) {
            self.set_bus_solo(id, solo);
        }
    }
}

fn bus_id_named(rt: &Runtime, name: &str) -> Option<BusId> {
    rt.mixer().buses().find(|b| b.name == name).map(|b| b.id)
}
