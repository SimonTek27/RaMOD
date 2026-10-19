//! S6.3 — Editor Live Update TCP client.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

const DEFAULT_ADDR: &str = "127.0.0.1:17420";

pub struct EditorLiveClient {
    stream: TcpStream,
    connected: bool,
}

impl EditorLiveClient {
    pub fn connect(addr: Option<&str>) -> Result<Self, String> {
        let addr = addr.unwrap_or(DEFAULT_ADDR);
        let stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_millis(300)))
            .ok();
        stream.set_nodelay(true).ok();
        let mut w = stream.try_clone().map_err(|e| e.to_string())?;
        writeln!(
            w,
            r#"{{"cmd":"hello","client":"ramod_editor","version":1}}"#
        )
        .map_err(|e| e.to_string())?;
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
        let mut line = String::new();
        let _ = reader.read_line(&mut line);
        Ok(Self {
            stream: w,
            connected: true,
        })
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn set_param(&mut self, event: &str, name: &str, value: f32) -> Result<(), String> {
        self.send(&format!(
            r#"{{"cmd":"set_param","event":"{event}","name":"{name}","value":{value}}}"#
        ))
    }

    pub fn set_bus_volume(&mut self, bus: &str, db: f32) -> Result<(), String> {
        self.send(&format!(
            r#"{{"cmd":"set_bus","bus":"{bus}","volume_db":{db}}}"#
        ))
    }

    pub fn ping(&mut self) -> Result<(), String> {
        self.send(r#"{"cmd":"ping"}"#)
    }

    fn send(&mut self, line: &str) -> Result<(), String> {
        match writeln!(self.stream, "{line}") {
            Ok(()) => Ok(()),
            Err(e) => {
                self.connected = false;
                Err(e.to_string())
            }
        }
    }
}
