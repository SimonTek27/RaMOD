//! FSB5 sound-bank container parsing.
//!
//! This module parses the FSB5 container that content-authoring tools use to
//! package audio samples for their event banks (`.fsb`). It is the foundation
//! of the RaMOD audio middleware: once samples and their metadata are decoded,
//! the modding suite can list, extract, replace and re-encode bank contents.
//!
//! The layout implemented here was cross-checked against the REA analysis of
//! the `libfsbvorbis64.dll` shipped with a stock authoring tool and the publicly
//! documented FSB5 binary template. Field names mirror that template so the
//! code stays traceable to the reverse-engineered source.
//!
//! FSB5 header (little-endian):
//! ```text
//! 0x00 char[4]  magic = "FSB5"
//! 0x04 u32      version
//! 0x08 u32      num_samples
//! 0x0C u32      sample_header_size   (total size of the sample-header region)
//! 0x10 u32      name_table_size
//! 0x14 u32      data_size
//! 0x18 u32      mode                (sound format)
//! 0x1C u8[8]    zero
//! 0x24 u8[16]   hash
//! 0x34 u8[8]    dummy
//! 0x3C u32      unknown             (only when version == 0)
//! ```

use std::fmt;

/// Magic bytes at the start of every FSB5 bank.
pub const FSB5_MAGIC: [u8; 4] = *b"FSB5";

/// Number of bytes occupied by the FSB5 header for version 1 (and later).
pub const FSB5_HEADER_SIZE: usize = 60;

/// Number of bytes occupied by the FSB5 header for version 0.
pub const FSB5_HEADER_SIZE_V0: usize = 64;

/// Errors produced while parsing an FSB5 container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsbError {
    /// The buffer is smaller than the structure being read.
    TooShort { needed: usize, got: usize },
    /// The first four bytes are not `FSB5`.
    BadMagic([u8; 4]),
    /// A read ran past the end of the buffer.
    Truncated {
        context: &'static str,
        offset: usize,
    },
    /// The declared number of samples cannot fit in the buffer.
    SampleCountTooLarge {
        samples: u32,
        available_bytes: usize,
    },
}

impl fmt::Display for FsbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsbError::TooShort { needed, got } => {
                write!(f, "buffer too short: needed {needed} bytes, got {got}")
            }
            FsbError::BadMagic(magic) => {
                write!(f, "not an FSB5 bank: magic bytes were {magic:?}")
            }
            FsbError::Truncated { context, offset } => {
                write!(f, "truncated while reading {context} at offset {offset}")
            }
            FsbError::SampleCountTooLarge {
                samples,
                available_bytes,
            } => write!(
                f,
                "declared {samples} samples but only {available_bytes} bytes are available"
            ),
        }
    }
}

impl std::error::Error for FsbError {}

// NOTE: Full module body restored from commit d0e38a1 + S0.4 patches.
// If this stub is present, the restore was truncated — re-run recovery.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_magic() {
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(b"NOPE");
        let error = Fsb5::parse(&data).unwrap_err();
        assert_eq!(error, FsbError::BadMagic(*b"NOPE"));
    }

    #[test]
    fn rejects_empty_buffer() {
        let error = Fsb5::parse(&[]).unwrap_err();
        assert!(matches!(error, FsbError::TooShort { .. }));
    }
}

/// Placeholder — see git history for full Fsb5 implementation.
#[derive(Debug)]
pub struct Fsb5;

impl Fsb5 {
    pub fn parse(_data: &[u8]) -> Result<Self, FsbError> {
        Err(FsbError::TooShort { needed: 60, got: 0 })
    }
}
