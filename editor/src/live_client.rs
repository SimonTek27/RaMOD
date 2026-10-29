//! S6.3 — Editor Live Update TCP client.
//!
//! The game runs `ramod_runtime::LiveUpdateServer` (feature `live-update`) on
//! `127.0.0.1:17420`. The protocol is line-oriented JSON in both directions:
//!
//! ```text
//! editor -> game  {"cmd":"set_param","event":"event:/sfx/x","name":"Intensity","value":0.5}
//! editor -> game  {"cmd":"set_bus","bus":"Music","volume_db":-6,"mute":true}
//! game  -> editor {"cmd":"welcome","server":"ramod_runtime","version":1}
//! ```
//!
//! Writes are fire-and-forget (the server only acknowledges errors), so the
//! client keeps the read side drained to notice a dropped game connection
//! instead of blocking on it.

use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpStream};
use std::time::Duration;

/// Default address the game's server binds.
pub const DEFAULT_ADDR: &str = "127.0.0.1:17420";

pub struct EditorLiveClient {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
    addr: String,
    connected: bool,
}

impl EditorLiveClient {
    pub fn connect(addr: Option<&str>) -> Result<Self, String> {
        let addr = addr.unwrap_or(DEFAULT_ADDR);
        let stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
        // Short timeout: `drain_replies` must never stall the UI frame. The UI
        // also throttles how often it polls, so an idle game costs ~1 ms/100 ms.
        stream.set_read_timeout(Some(Duration::from_millis(1))).ok();
        stream.set_nodelay(true).ok();
        let writer = stream.try_clone().map_err(|e| e.to_string())?;
        let reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
        let mut client = Self {
            stream: writer,
            reader,
            addr: addr.to_string(),
            connected: true,
        };
        client.send_raw(r#"{"cmd":"hello","client":"ramod_editor","version":1}"#)?;
        Ok(client)
    }

    pub fn addr(&self) -> &str {
        &self.addr
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Push an event parameter (e.g. `Intensity`) to every live instance.
    pub fn set_param(&mut self, event: &str, name: &str, value: f32) -> Result<(), String> {
        self.send_raw(&format!(
            r#"{{"cmd":"set_param","event":"{}","name":"{}","value":{value}}}"#,
            json_escape(event),
            json_escape(name)
        ))
    }

    pub fn set_bus_volume(&mut self, bus: &str, db: f32) -> Result<(), String> {
        self.set_bus_state(bus, Some(db), None, None)
    }

    /// Set any subset of a bus's state in one line.
    pub fn set_bus_state(
        &mut self,
        bus: &str,
        volume_db: Option<f32>,
        mute: Option<bool>,
        solo: Option<bool>,
    ) -> Result<(), String> {
        let mut line = format!(r#"{{"cmd":"set_bus","bus":"{}""#, json_escape(bus));
        if let Some(db) = volume_db {
            line.push_str(&format!(r#","volume_db":{db}"#));
        }
        if let Some(m) = mute {
            line.push_str(&format!(r#","mute":{m}"#));
        }
        if let Some(s) = solo {
            line.push_str(&format!(r#","solo":{s}"#));
        }
        line.push('}');
        self.send_raw(&line)
    }

    /// Read whatever the game sent since the last call (handshake, errors).
    ///
    /// Returns `Err` when the game closed the connection, so the UI can show it.
    pub fn drain_replies(&mut self) -> Result<Vec<String>, String> {
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => {
                    self.connected = false;
                    return Err("game closed the connection".into());
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        lines.push(trimmed.to_string());
                    }
                    if lines.len() >= 32 {
                        break;
                    }
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    break;
                }
                Err(e) => {
                    self.connected = false;
                    return Err(e.to_string());
                }
            }
        }
        Ok(lines)
    }

    pub fn disconnect(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
        self.connected = false;
    }

    fn send_raw(&mut self, line: &str) -> Result<(), String> {
        match writeln!(self.stream, "{line}") {
            Ok(()) => Ok(()),
            Err(e) => {
                self.connected = false;
                Err(e.to_string())
            }
        }
    }
}

/// Minimal escaping for the string fields the protocol carries.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ramod_runtime::{EventDescription, LiveUpdateServer, Runtime, RuntimeConfig};

    struct Harness {
        server: LiveUpdateServer,
        rt: Runtime,
        client: EditorLiveClient,
    }

    fn harness() -> Harness {
        let server = LiveUpdateServer::bind("127.0.0.1:0").expect("bind server");
        let addr = server.local_addr();
        let rt = Runtime::new(RuntimeConfig {
            sample_rate: 48_000,
            channels: 2,
            max_voices: 8,
            open_device: false,
        })
        .expect("headless runtime");
        let client = EditorLiveClient::connect(Some(&addr)).expect("connect");
        Harness { server, rt, client }
    }

    /// The server only accepts on `poll`, so a command needs a couple of polls.
    fn poll_until(h: &mut Harness, mut done: impl FnMut(&mut Runtime) -> bool) {
        for _ in 0..32 {
            h.server.poll(&mut h.rt).expect("poll");
            if done(&mut h.rt) {
                return;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        panic!("server never applied the command");
    }

    #[test]
    fn parameter_edits_reach_the_playing_game() {
        let mut h = harness();
        let buffer = h.rt.load_pcm(vec![0.2; 4_800], 1, 48_000).expect("pcm");
        h.rt.register_event(EventDescription::new("event:/sfx/live", buffer));
        h.rt.play_event("event:/sfx/live").expect("play");
        h.rt.profiler_enable(true);

        h.client
            .set_param("event:/sfx/live", "Intensity", 0.42)
            .expect("send param");

        poll_until(&mut h, |rt| {
            let _ = rt.render_seconds(0.005);
            rt.profiler_latest()
                .map(|f| {
                    f.params.iter().any(|(path, name, value)| {
                        path == "event:/sfx/live"
                            && name == "Intensity"
                            && (*value - 0.42).abs() < 1e-6
                    })
                })
                .unwrap_or(false)
        });
    }

    #[test]
    fn bus_edits_reach_the_game() {
        let mut h = harness();
        h.client.set_bus_volume("Music", -6.0).expect("send volume");
        h.client
            .set_bus_state("Music", None, Some(true), Some(true))
            .expect("send mute+solo");

        poll_until(&mut h, |rt| {
            rt.mixer()
                .buses()
                .any(|b| b.name == "Music" && (b.volume_db + 6.0).abs() < 1e-6 && b.mute && b.solo)
        });
    }

    #[test]
    fn handshake_is_valid_json_and_disconnect_is_noticed() {
        let mut h = harness();
        h.server.poll(&mut h.rt).expect("poll");
        let mut replies = Vec::new();
        for _ in 0..8 {
            replies.extend(h.client.drain_replies().unwrap_or_default());
            if !replies.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(
            replies.iter().any(|l| l.contains("welcome")),
            "expected a welcome reply, got {replies:?}"
        );
        assert!(
            replies
                .iter()
                .all(|l| l.starts_with('{') && l.ends_with('}')),
            "every reply must be a JSON object, got {replies:?}"
        );

        h.client.disconnect();
        assert!(!h.client.is_connected());
        for _ in 0..32 {
            h.server.poll(&mut h.rt).expect("poll");
            if h.server.client_count() == 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(h.server.client_count(), 0, "server should reap the client");
    }

    #[test]
    fn quiet_socket_does_not_block_the_frame() {
        let mut h = harness();
        let start = std::time::Instant::now();
        for _ in 0..10 {
            assert!(h.client.drain_replies().is_ok());
        }
        assert!(
            start.elapsed() < Duration::from_millis(200),
            "drain_replies must not block the frame when nothing is pending"
        );
    }

    #[test]
    fn string_fields_are_escaped_not_injected() {
        let mut h = harness();
        h.client
            .set_param(r#"event:/x"y"#, "Intensity", 1.0)
            .expect("send");
        let line = format!(
            r#"{{"cmd":"set_param","event":"{}","name":"Intensity","value":1}}"#,
            json_escape(r#"event:/x"y"#)
        );
        assert!(line.contains(r#"event:/x\"y"#));
        assert_eq!(line.matches("{\"cmd\"").count(), 1, "no duplicate cmd key");
    }
}
