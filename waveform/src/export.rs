//! WAV export (S9.4).

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

#[derive(Debug)]
pub enum WavError {
    Io(io::Error),
    Empty,
}

impl std::fmt::Display for WavError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WavError::Io(e) => write!(f, "io: {e}"),
            WavError::Empty => write!(f, "empty buffer"),
        }
    }
}

impl std::error::Error for WavError {}

impl From<io::Error> for WavError {
    fn from(e: io::Error) -> Self {
        WavError::Io(e)
    }
}

/// Write interleaved f32 samples as 16-bit PCM WAV.
pub fn write_wav_i16(
    path: &Path,
    samples: &[f32],
    channels: u16,
    sample_rate: u32,
) -> Result<(), WavError> {
    if samples.is_empty() || channels == 0 || sample_rate == 0 {
        return Err(WavError::Empty);
    }
    let ch = channels as usize;
    let frames = samples.len() / ch;
    let data_bytes = frames * ch * 2;
    let mut file = File::create(path)?;

    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_bytes as u32).to_le_bytes())?;
    file.write_all(b"WAVE")?;
    // fmt
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // chunk size
    file.write_all(&1u16.to_le_bytes())?; // PCM
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    let byte_rate = sample_rate as u32 * channels as u32 * 2;
    file.write_all(&byte_rate.to_le_bytes())?;
    let block_align = channels * 2;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?; // bits
                                           // data
    file.write_all(b"data")?;
    file.write_all(&(data_bytes as u32).to_le_bytes())?;

    for &s in samples.iter().take(frames * ch) {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        file.write_all(&v.to_le_bytes())?;
    }
    Ok(())
}
