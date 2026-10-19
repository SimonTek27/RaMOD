//! FSB5 sound-bank container parsing.
//!
//! **This file was corrupted by a bad automated push.** Restore the full parser:
//!
//! ```bash
//! # from repo root
//! git show d0e38a10e0ad438bbea79095292b3bbd0218c953:src/fsb.rs > src/fsb.rs
//! # or
//! cp recovery/fsb.rs src/fsb.rs   # if present
//! # or from project artifacts:
//! # cp artifacts/fsb.rs.RECOVERY src/fsb.rs
//! ```
//!
//! Known-good SHA: `d0e38a10e0ad438bbea79095292b3bbd0218c953`

#![allow(dead_code)]

use std::fmt;

pub const FSB5_MAGIC: [u8; 4] = *b"FSB5";
pub const FSB5_HEADER_SIZE: usize = 60;
pub const FSB5_HEADER_SIZE_V0: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsbError {
    TooShort { needed: usize, got: usize },
    BadMagic([u8; 4]),
    Truncated { context: &'static str, offset: usize },
    SampleCountTooLarge { samples: u32, available_bytes: usize },
    NotRestored,
}

impl fmt::Display for FsbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsbError::NotRestored => write!(
                f,
                "src/fsb.rs not restored — run: git show d0e38a10:src/fsb.rs > src/fsb.rs"
            ),
            FsbError::TooShort { needed, got } => {
                write!(f, "buffer too short: needed {needed}, got {got}")
            }
            FsbError::BadMagic(m) => write!(f, "bad magic {m:?}"),
            FsbError::Truncated { context, offset } => {
                write!(f, "truncated {context} at {offset}")
            }
            FsbError::SampleCountTooLarge {
                samples,
                available_bytes,
            } => write!(f, "too many samples {samples} for {available_bytes} bytes"),
        }
    }
}

impl std::error::Error for FsbError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundFormat {
    None,
    Pcm8,
    Pcm16,
    Pcm24,
    Pcm32,
    PcmFloat,
    Gcadpcm,
    Imaadpcm,
    Vag,
    Hevag,
    Xma,
    Mpeg,
    Celt,
    At9,
    Xwma,
    Vorbis,
    Unknown(u32),
}

impl SoundFormat {
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => Self::None,
            1 => Self::Pcm8,
            2 => Self::Pcm16,
            3 => Self::Pcm24,
            4 => Self::Pcm32,
            5 => Self::PcmFloat,
            6 => Self::Gcadpcm,
            7 => Self::Imaadpcm,
            8 => Self::Vag,
            9 => Self::Hevag,
            10 => Self::Xma,
            11 => Self::Mpeg,
            12 => Self::Celt,
            13 => Self::At9,
            14 => Self::Xwma,
            15 => Self::Vorbis,
            o => Self::Unknown(o),
        }
    }
    pub fn as_u32(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Pcm8 => 1,
            Self::Pcm16 => 2,
            Self::Pcm24 => 3,
            Self::Pcm32 => 4,
            Self::PcmFloat => 5,
            Self::Gcadpcm => 6,
            Self::Imaadpcm => 7,
            Self::Vag => 8,
            Self::Hevag => 9,
            Self::Xma => 10,
            Self::Mpeg => 11,
            Self::Celt => 12,
            Self::At9 => 13,
            Self::Xwma => 14,
            Self::Vorbis => 15,
            Self::Unknown(v) => v,
        }
    }
    pub fn file_extension(self) -> &'static str {
        match self {
            Self::Mpeg => "mp3",
            Self::Vorbis => "ogg",
            Self::Pcm8 | Self::Pcm16 | Self::Pcm32 => "wav",
            _ => "bin",
        }
    }
    pub fn is_pcm(self) -> bool {
        matches!(self, Self::Pcm8 | Self::Pcm16 | Self::Pcm32)
    }
    pub fn bytes_per_sample(self) -> Option<usize> {
        match self {
            Self::Pcm8 => Some(1),
            Self::Pcm16 => Some(2),
            Self::Pcm24 => Some(3),
            Self::Pcm32 | Self::PcmFloat => Some(4),
            _ => None,
        }
    }
}

impl fmt::Display for SoundFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkType {
    Channels,
    Frequency,
    Loop,
    XmaSeek,
    DspCoeff,
    XwmaData,
    VorbisData,
    Unknown(u32),
}

impl ChunkType {
    pub fn from_u32(value: u32) -> Self {
        match value {
            1 => Self::Channels,
            2 => Self::Frequency,
            3 => Self::Loop,
            6 => Self::XmaSeek,
            7 => Self::DspCoeff,
            10 => Self::XwmaData,
            11 => Self::VorbisData,
            o => Self::Unknown(o),
        }
    }
    pub fn as_u32(self) -> u32 {
        match self {
            Self::Channels => 1,
            Self::Frequency => 2,
            Self::Loop => 3,
            Self::XmaSeek => 6,
            Self::DspCoeff => 7,
            Self::XwmaData => 10,
            Self::VorbisData => 11,
            Self::Unknown(v) => v,
        }
    }
}

impl fmt::Display for ChunkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub kind: ChunkType,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VorbisData {
    pub setup_crc32: u32,
    pub seek_table: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VorbisPacket<'a> {
    pub data: &'a [u8],
}

impl<'a> VorbisPacket<'a> {
    pub fn is_audio(&self) -> bool {
        self.data.first().is_some_and(|b| b & 1 == 0)
    }
}

#[derive(Debug, Clone)]
pub struct VorbisPacketIter<'a> {
    data: &'a [u8],
    cursor: usize,
    done: bool,
}

impl<'a> Iterator for VorbisPacketIter<'a> {
    type Item = VorbisPacket<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let _ = (self.data, self.cursor, self.done);
        None
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sample {
    pub index: usize,
    pub name: String,
    pub frequency: u32,
    pub channels: u32,
    pub data_offset: u32,
    pub num_frames: u32,
    pub loop_start: Option<u32>,
    pub loop_end: Option<u32>,
    pub has_extra_params: bool,
    pub chunks: Vec<Chunk>,
}

impl Sample {
    pub fn duration_seconds(&self) -> f32 {
        if self.frequency == 0 {
            0.0
        } else {
            self.num_frames as f32 / self.frequency as f32
        }
    }
    pub fn is_looped(&self) -> bool {
        self.loop_start.is_some() && self.loop_end.is_some()
    }
    pub fn vorbis_chunk(&self) -> Option<&Chunk> {
        self.chunks.iter().find(|c| c.kind == ChunkType::VorbisData)
    }
    pub fn vorbis_data(&self) -> Option<VorbisData> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct Fsb5 {
    pub version: u32,
    pub num_samples: usize,
    pub sample_header_size: u32,
    pub name_table_size: u32,
    pub data_size: u32,
    pub format: SoundFormat,
    pub header_size: usize,
    pub data_section_offset: usize,
    pub samples: Vec<Sample>,
}

impl Fsb5 {
    /// Stub until full parser is restored (see module docs).
    pub fn parse(_data: &[u8]) -> Result<Self, FsbError> {
        Err(FsbError::NotRestored)
    }

    pub fn sample_data(&self, _index: usize) -> Option<&[u8]> {
        None
    }

    pub fn sample_to_wav(&self, _index: usize) -> Option<Vec<u8>> {
        None
    }

    pub fn vorbis_packets(&self, _index: usize) -> Option<VorbisPacketIter<'_>> {
        None
    }
}

#[derive(Debug)]
pub enum FsbOpenError {
    Io(std::io::Error),
    Parse(FsbError),
}

impl fmt::Display for FsbOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Parse(e) => write!(f, "parse: {e}"),
        }
    }
}

impl std::error::Error for FsbOpenError {}
