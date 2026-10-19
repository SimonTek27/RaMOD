//! S6 — Live Update TCP server (feature `live-update`).
//!
//! Line-oriented JSON protocol — see docs/LIVE_UPDATE_PROTOCOL.md.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::mixer::BusId;
use crate::Runtime;

const DEFAULT_ADDR: &str = "127.0.0.1:17420";

/// Live Update server: game listens, editor connects.
pub struct LiveUpdateServer {
    listener: TcpListener,
    clients: Vec<ClientConn>,
}

struct ClientConn {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
    buf: String,
}

impl LiveUpdateServer {
    pub fn bind_default() -> Result<Self, String> {
        Self::bind(DEFAULT_ADDR)
    }

    pub fn bind(addr: impl ToSocketAddrs) -> Result<Self, String> {
        let listener = TcpListener::bind(addr).map_err(|e| e.to_string())?;
        listener
            .set_nonblocking(true)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            listener,
            clients: Vec::new(),
        })
    }

    pub fn local_addr(&self) -> String {
        self.listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| DEFAULT_ADDR.into())
    }

    /// Accept new clients and process pending lines.
    pub fn poll(&mut self, rt: &mut Runtime) -> Result<(), String> {
        // Accept
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    stream.set_nonblocking(true).ok();
                    stream
                        .set_read_timeout(Some(Duration::from_millis(1)))
                        .ok();
                    let mut stream_write = stream.try_clone().map_err(|e| e.to_string())?;
                    let welcome = r#{"cmd":"welcome","server":"ramod_runtime","version":1,"features":["param","bus","snapshot"]}"#;
                    let _ = writeln!(stream_write, "{welcome}");
                    let reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
                    self.clients.push(ClientConn {
                        stream: stream_write,
                        reader,
                        buf: String::new(),
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.to_string()),
            }
        }

        // Read lines
        let mut dead = Vec::new();
        for (i, client) in self.clients.iter_mut().enumerate() {
            loop {
                client.buf.clear();
                match client.reader.read_line(&mut client.buf) {
                    Ok(0) => {
                        dead.push(i);
                        break;
                    }
                    Ok(_) => {
                        let line = client.buf.trim();
                        if line.is_empty() {
                            continue;
                        }
                        let reply = handle_line(rt, line);
                        if let Some(r) = reply {
                            let _ = writeln!(client.stream, "{r}");
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => {
                        dead.push(i);
                        break;
                    }
                }
            }
        }
        for i in dead.into_iter().rev() {
            self.clients.swap_remove(i);
        }
        Ok(())
    }

    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
}

fn handle_line(rt: &mut Runtime, line: &str) -> Option<String> {
    // Minimal JSON field extraction without serde (keep runtime lean).
    let cmd = json_str(line, "cmd").unwrap_or("");
    match cmd {
        "hello" => Some(
            r#{"cmd":"welcome","server":"ramod_runtime","version":1,"features":["param","bus","snapshot"]}"#
                .into(),
        ),
        "ping" => Some(r#{"cmd":"pong"}"#.into()),
        "set_param" => {
            let event = json_str(line, "event").unwrap_or("");
            let name = json_str(line, "name").unwrap_or("");
            let value = json_f32(line, "value").unwrap_or(0.0);
            rt.live_set_param(event, name, value);
            None
        }
        "set_bus" => {
            let bus = json_str(line, "bus").unwrap_or("");
            if let Some(db) = json_f32(line, "volume_db") {
                rt.live_set_bus_volume(bus, db);
            }
            if let Some(m) = json_bool(line, "mute") {
                rt.live_set_bus_mute(bus, m);
            }
            if let Some(s) = json_bool(line, "solo") {
                rt.live_set_bus_solo(bus, s);
            }
            None
        }
        "snapshot" => {
            let name = json_str(line, "name").unwrap_or("");
            let w = json_f32(line, "weight").unwrap_or(1.0);
            rt.trigger_snapshot(name, w);
            None
        }
        "snapshot_clear" => {
            rt.clear_snapshot();
            None
        }
        _ => Some(format!(
            r#"{{"cmd":"error","message":"unknown cmd"}}"#
        )),
    }
}

fn json_str<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":");
    let i = line.find(&pat)?;
    let rest = &line[i + pat.len()..];
    let rest = rest.trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let rest = &rest[1..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

fn json_f32(line: &str, key: &str) -> Option<f32> {
    let pat = format!("\"{key}\":");
    let i = line.find(&pat)?;
    let rest = line[i + pat.len()..].trim_start();
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-' || *c == '.' || *c == 'e' || *c == 'E')
        .collect();
    num.parse().ok()
}

fn json_bool(line: &str, key: &str) -> Option<bool> {
    let pat = format!("\"{key}\":");
    let i = line.find(&pat)?;
    let rest = line[i + pat.len()..].trim_start();
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

/// Client helper for tools / tests.
pub struct LiveUpdateClient {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
}

impl LiveUpdateClient {
    pub fn connect(addr: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .ok();
        let mut stream_w = stream.try_clone().map_err(|e| e.to_string())?;
        writeln!(stream_w, r#"{{"cmd":"hello","client":"tool","version":1}}"#)
            .map_err(|e| e.to_string())?;
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
        let mut line = String::new();
        let _ = reader.read_line(&mut line);
        Ok(Self {
            stream: stream_w,
            reader,
        })
    }

    pub fn send_raw(&mut self, line: &str) -> Result<(), String> {
        writeln!(self.stream, "{line}").map_err(|e| e.to_string())
    }

    pub fn set_param(&mut self, event: &str, name: &str, value: f32) -> Result<(), String> {
        self.send_raw(&format!(
            r#"{{"cmd":"set_param","event":"{event}","name":"{name}","value":{value}}}"#
        ))
    }

    pub fn set_bus_volume(&mut self, bus: &str, volume_db: f32) -> Result<(), String> {
        self.send_raw(&format!(
            r#"{{"cmd":"set_bus","bus":"{bus}","volume_db":{volume_db}}}"#
        ))
    }

    pub fn ping(&mut self) -> Result<String, String> {
        self.send_raw(r#"{"cmd":"ping"}"#)?;
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        Ok(line)
    }
}

/// Resolve bus name to id.
pub fn bus_id_by_name(rt: &Runtime, name: &str) -> Option<BusId> {
    rt.mixer().buses().find(|b| b.name == name).map(|b| b.id)
}
