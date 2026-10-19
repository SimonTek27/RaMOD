//! Unified error surface for library consumers.
//!
//! Domain modules keep their specific enums ([`crate::FsbError`],
//! [`crate::VorbisExportError`], …). This module adds a single
//! [`RamodError`] that folds them together for CLI and application code,
//! with proper [`std::error::Error::source`] chains.

use std::fmt;

use crate::fsb::{FsbError, FsbOpenError};
use crate::fsb_write::{FsbWriteError, WavError};
use crate::vorbis::VorbisExportError;

#[cfg(feature = "decoder-rust")]
use crate::vorbis_decode::VorbisDecodeError;

/// Top-level error for operations that may touch several RaMOD layers.
#[derive(Debug)]
pub enum RamodError {
    /// Filesystem / I/O failure.
    Io(std::io::Error),
    /// FSB5 parse failure.
    Fsb(FsbError),
    /// Open path that mixes I/O and parse.
    Open(FsbOpenError),
    /// Bank / WAV write path.
    Write(FsbWriteError),
    /// WAV-only parse failure.
    Wav(WavError),
    /// Vorbis import / export / encode.
    Vorbis(VorbisExportError),
    /// Pure-Rust decode path.
    #[cfg(feature = "decoder-rust")]
    Decode(VorbisDecodeError),
    /// Catch-all message when no richer type applies.
    Message(String),
}

impl fmt::Display for RamodError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RamodError::Io(e) => write!(f, "I/O error: {e}"),
            RamodError::Fsb(e) => write!(f, "FSB5 error: {e}"),
            RamodError::Open(e) => write!(f, "open bank: {e}"),
            RamodError::Write(e) => write!(f, "write bank: {e}"),
            RamodError::Wav(e) => write!(f, "WAV error: {e}"),
            RamodError::Vorbis(e) => write!(f, "Vorbis error: {e}"),
            #[cfg(feature = "decoder-rust")]
            RamodError::Decode(e) => write!(f, "decode error: {e}"),
            RamodError::Message(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for RamodError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RamodError::Io(e) => Some(e),
            RamodError::Fsb(e) => Some(e),
            RamodError::Open(e) => Some(e),
            RamodError::Write(e) => Some(e),
            RamodError::Wav(e) => Some(e),
            RamodError::Vorbis(e) => Some(e),
            #[cfg(feature = "decoder-rust")]
            RamodError::Decode(e) => Some(e),
            RamodError::Message(_) => None,
        }
    }
}

impl From<std::io::Error> for RamodError {
    fn from(value: std::io::Error) -> Self {
        RamodError::Io(value)
    }
}

impl From<FsbError> for RamodError {
    fn from(value: FsbError) -> Self {
        RamodError::Fsb(value)
    }
}

impl From<FsbOpenError> for RamodError {
    fn from(value: FsbOpenError) -> Self {
        RamodError::Open(value)
    }
}

impl From<FsbWriteError> for RamodError {
    fn from(value: FsbWriteError) -> Self {
        RamodError::Write(value)
    }
}

impl From<WavError> for RamodError {
    fn from(value: WavError) -> Self {
        RamodError::Wav(value)
    }
}

impl From<VorbisExportError> for RamodError {
    fn from(value: VorbisExportError) -> Self {
        RamodError::Vorbis(value)
    }
}

#[cfg(feature = "decoder-rust")]
impl From<VorbisDecodeError> for RamodError {
    fn from(value: VorbisDecodeError) -> Self {
        RamodError::Decode(value)
    }
}

impl From<String> for RamodError {
    fn from(value: String) -> Self {
        RamodError::Message(value)
    }
}

impl From<&str> for RamodError {
    fn from(value: &str) -> Self {
        RamodError::Message(value.to_owned())
    }
}

/// Walk the `source()` chain into a single multi-line string (for CLI logs).
pub fn format_error_chain(err: &dyn std::error::Error) -> String {
    let mut out = err.to_string();
    let mut current = err.source();
    while let Some(src) = current {
        out.push_str("\n  caused by: ");
        out.push_str(&src.to_string());
        current = src.source();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fsb_error_display_is_actionable() {
        let e = FsbError::TooShort {
            needed: 60,
            got: 4,
        };
        let s = e.to_string();
        assert!(s.contains("60"));
        assert!(s.contains("4"));
    }

    #[test]
    fn setup_crc_mentions_hex() {
        let e = VorbisExportError::UnknownSetupHeader(0xc2ea_917f);
        let s = e.to_string();
        assert!(
            s.contains("c2ea917f") || s.contains("C2EA917F") || s.contains("0xc2ea917f"),
            "display should show hex CRC, got: {s}"
        );
    }

    #[test]
    fn open_error_source_is_parse() {
        let inner = FsbError::BadMagic(*b"NOPE");
        let open = FsbOpenError::Parse(inner.clone());
        let src = std::error::Error::source(&open).expect("source");
        assert_eq!(src.to_string(), inner.to_string());
    }

    #[test]
    fn ramod_error_chains_write() {
        let w = FsbWriteError::NoSamples;
        let r = RamodError::from(w);
        assert!(r.to_string().contains("write") || r.to_string().contains("sample"));
        assert!(std::error::Error::source(&r).is_some());
    }

    #[test]
    fn format_error_chain_lists_causes() {
        let open = FsbOpenError::Parse(FsbError::BadMagic(*b"XXXX"));
        let chain = format_error_chain(&open);
        assert!(chain.contains("caused by") || chain.contains("NOPE") || chain.contains("XXXX"));
    }
}
