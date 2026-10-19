//! WAV import (PCM 16-bit / IEEE float, little-endian).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::{WaveBuffer, WaveError};

#[derive(Debug)]
pub enum ImportError {
    Io(std::io::Error),
    Format(&'static str),
    Wave(WaveError),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Io(e) => write!(f, "io: {e}"),
            ImportError::Format(m) => write!(f, "format: {m}"),
            ImportError::Wave(e) => write!(f, "wave: {e}"),
        }
    }
}

impl std::error::Error for ImportError {}

impl From<std::io::Error> for ImportError {
    fn from(e: std::io::Error) -> Self {
        ImportError::Io(e)
    }
}

/// Load a WAV file into a [`WaveBuffer`].
pub fn read_wav(path: &Path) -> Result<WaveBuffer, ImportError> {
    let mut f = File::open(path)?;
    let mut header = [0u8; 12];
    f.read_exact(&mut header)?;
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return Err(ImportError::Format("not a RIFF/WAVE file"));
    }

    let mut channels = 0u16;
    let mut sample_rate = 0u32;
    let mut bits = 0u16;
    let mut audio_format = 0u16; // 1 = PCM, 3 = IEEE float
    let mut data: Vec<u8> = Vec::new();

    loop {
        let mut chunk_hdr = [0u8; 8];
        match f.read_exact(&mut chunk_hdr) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
        let id = &chunk_hdr[0..4];
        let size = u32::from_le_bytes(chunk_hdr[4..8].try_into().unwrap()) as u64;

        if id == b"fmt " {
            let mut fmt = vec![0u8; size as usize];
            f.read_exact(&mut fmt)?;
            if fmt.len() < 16 {
                return Err(ImportError::Format("fmt chunk too small"));
            }
            audio_format = u16::from_le_bytes(fmt[0..2].try_into().unwrap());
            channels = u16::from_le_bytes(fmt[2..4].try_into().unwrap());
            sample_rate = u32::from_le_bytes(fmt[4..8].try_into().unwrap());
            bits = u16::from_le_bytes(fmt[14..16].try_into().unwrap());
            // pad byte if odd size
            if size % 2 == 1 {
                let _ = f.seek(SeekFrom::Current(1));
            }
        } else if id == b"data" {
            data.resize(size as usize, 0);
            f.read_exact(&mut data)?;
            if size % 2 == 1 {
                let _ = f.seek(SeekFrom::Current(1));
            }
            break; // got data
        } else {
            f.seek(SeekFrom::Current(size as i64))?;
            if size % 2 == 1 {
                let _ = f.seek(SeekFrom::Current(1));
            }
        }
    }

    if channels == 0 || sample_rate == 0 || data.is_empty() {
        return Err(ImportError::Format("missing fmt/data"));
    }

    let samples = match (audio_format, bits) {
        (1, 16) => {
            let mut out = Vec::with_capacity(data.len() / 2);
            for chunk in data.chunks_exact(2) {
                let v = i16::from_le_bytes([chunk[0], chunk[1]]);
                out.push(v as f32 / 32768.0);
            }
            out
        }
        (1, 8) => {
            // unsigned 8-bit
            data.iter().map(|&b| (b as f32 - 128.0) / 128.0).collect()
        }
        (3, 32) => {
            let mut out = Vec::with_capacity(data.len() / 4);
            for chunk in data.chunks_exact(4) {
                let v = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                out.push(v);
            }
            out
        }
        _ => {
            return Err(ImportError::Format(
                "unsupported format (need PCM16/PCM8 or float32)",
            ))
        }
    };

    WaveBuffer::new(samples, channels, sample_rate).map_err(ImportError::Wave)
}
