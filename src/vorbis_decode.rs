//! Pure-Rust Vorbis → PCM decoding (`lewton`).
//!
//! Enabled with the `decoder-rust` feature (default).
//!
//! Typical path for an FSB5 sample:
//! 1. [`crate::build_ogg`] reconstructs a playable Ogg stream
//! 2. [`decode_ogg_to_pcm`] decodes to interleaved `f32` PCM

use std::io::Cursor;

use lewton::inside_ogg::OggStreamReader;

use crate::fsb::Fsb5;
use crate::vorbis::{build_ogg, VorbisExportError};

/// Errors from the pure-Rust decoder path.
#[derive(Debug)]
pub enum VorbisDecodeError {
    /// Ogg rebuild from the bank failed.
    Export(VorbisExportError),
    /// `lewton` rejected the stream.
    Decode(String),
    /// Empty or unusable PCM output.
    Empty,
}

impl std::fmt::Display for VorbisDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VorbisDecodeError::Export(e) => write!(f, "ogg export: {e}"),
            VorbisDecodeError::Decode(e) => write!(f, "decode: {e}"),
            VorbisDecodeError::Empty => f.write_str("decoder produced no samples"),
        }
    }
}

impl std::error::Error for VorbisDecodeError {}

impl From<VorbisExportError> for VorbisDecodeError {
    fn from(value: VorbisExportError) -> Self {
        VorbisDecodeError::Export(value)
    }
}

/// Interleaved `f32` PCM in approximately [-1, 1].
#[derive(Debug, Clone)]
pub struct DecodedPcm {
    /// Channel count.
    pub channels: u32,
    /// Sample rate in hertz.
    pub sample_rate: u32,
    /// Interleaved samples (`frame0_ch0, frame0_ch1, …`).
    pub samples: Vec<f32>,
}

impl DecodedPcm {
    /// Number of frames (samples per channel).
    pub fn frames(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / self.channels as usize
        }
    }
}

/// Decode a complete Ogg Vorbis byte stream to interleaved `f32` PCM.
pub fn decode_ogg_to_pcm(ogg: &[u8]) -> Result<DecodedPcm, VorbisDecodeError> {
    let cursor = Cursor::new(ogg);
    let mut reader = OggStreamReader::new(cursor)
        .map_err(|e| VorbisDecodeError::Decode(format!("ogg open: {e}")))?;

    let channels = reader.ident_hdr.audio_channels as u32;
    let sample_rate = reader.ident_hdr.audio_sample_rate;
    if channels == 0 || sample_rate == 0 {
        return Err(VorbisDecodeError::Decode(
            "invalid ident header (channels/rate)".into(),
        ));
    }

    let mut samples: Vec<f32> = Vec::new();
    while let Some(packet) = reader
        .read_dec_packet_itl()
        .map_err(|e| VorbisDecodeError::Decode(format!("packet: {e}")))?
    {
        for s in packet {
            samples.push(s as f32 / 32768.0);
        }
    }

    if samples.is_empty() {
        return Err(VorbisDecodeError::Empty);
    }

    Ok(DecodedPcm {
        channels,
        sample_rate,
        samples,
    })
}

/// Decode one FSB5 `VORBIS` sample to interleaved `f32` PCM.
///
/// Rebuilds Ogg via the bundled setup-header table, then runs `lewton`.
pub fn decode_sample_to_pcm(
    bank: &Fsb5,
    index: usize,
) -> Result<DecodedPcm, VorbisDecodeError> {
    let ogg = build_ogg(bank, index)?;
    decode_ogg_to_pcm(&ogg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsb::{Fsb5, SoundFormat};

    #[test]
    fn api_linked() {
        let _ = std::mem::size_of::<DecodedPcm>();
        let _ = SoundFormat::Vorbis;
        let _ = std::any::type_name::<Fsb5>();
    }
}
