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

/// Audio sample encoding declared by an FSB5 bank.
///
/// Values match the `MODE` enum in the FSB5 specification.
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
    /// A mode value not covered by the specification.
    Unknown(u32),
}

impl SoundFormat {
    /// Decode the on-disk `mode` field.
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => SoundFormat::None,
            1 => SoundFormat::Pcm8,
            2 => SoundFormat::Pcm16,
            3 => SoundFormat::Pcm24,
            4 => SoundFormat::Pcm32,
            5 => SoundFormat::PcmFloat,
            6 => SoundFormat::Gcadpcm,
            7 => SoundFormat::Imaadpcm,
            8 => SoundFormat::Vag,
            9 => SoundFormat::Hevag,
            10 => SoundFormat::Xma,
            11 => SoundFormat::Mpeg,
            12 => SoundFormat::Celt,
            13 => SoundFormat::At9,
            14 => SoundFormat::Xwma,
            15 => SoundFormat::Vorbis,
            other => SoundFormat::Unknown(other),
        }
    }

    /// Encode back to the on-disk `mode` field.
    pub fn as_u32(self) -> u32 {
        match self {
            SoundFormat::None => 0,
            SoundFormat::Pcm8 => 1,
            SoundFormat::Pcm16 => 2,
            SoundFormat::Pcm24 => 3,
            SoundFormat::Pcm32 => 4,
            SoundFormat::PcmFloat => 5,
            SoundFormat::Gcadpcm => 6,
            SoundFormat::Imaadpcm => 7,
            SoundFormat::Vag => 8,
            SoundFormat::Hevag => 9,
            SoundFormat::Xma => 10,
            SoundFormat::Mpeg => 11,
            SoundFormat::Celt => 12,
            SoundFormat::At9 => 13,
            SoundFormat::Xwma => 14,
            SoundFormat::Vorbis => 15,
            SoundFormat::Unknown(value) => value,
        }
    }

    /// Conventional file extension for extracted sample data.
    pub fn file_extension(self) -> &'static str {
        match self {
            SoundFormat::Mpeg => "mp3",
            SoundFormat::Vorbis => "ogg",
            SoundFormat::Pcm8 | SoundFormat::Pcm16 | SoundFormat::Pcm32 => "wav",
            _ => "bin",
        }
    }

    /// Whether the format is uncompressed linear PCM.
    pub fn is_pcm(self) -> bool {
        matches!(
            self,
            SoundFormat::Pcm8 | SoundFormat::Pcm16 | SoundFormat::Pcm32
        )
    }

    /// Bytes per sample for the uncompressed formats, if applicable.
    pub fn bytes_per_sample(self) -> Option<usize> {
        match self {
            SoundFormat::Pcm8 => Some(1),
            SoundFormat::Pcm16 => Some(2),
            SoundFormat::Pcm24 => Some(3),
            SoundFormat::Pcm32 | SoundFormat::PcmFloat => Some(4),
            _ => None,
        }
    }
}

impl fmt::Display for SoundFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            SoundFormat::None => "none",
            SoundFormat::Pcm8 => "pcm8",
            SoundFormat::Pcm16 => "pcm16",
            SoundFormat::Pcm24 => "pcm24",
            SoundFormat::Pcm32 => "pcm32",
            SoundFormat::PcmFloat => "pcm_float",
            SoundFormat::Gcadpcm => "gcadpcm",
            SoundFormat::Imaadpcm => "imaadpcm",
            SoundFormat::Vag => "vag",
            SoundFormat::Hevag => "hevag",
            SoundFormat::Xma => "xma",
            SoundFormat::Mpeg => "mpeg",
            SoundFormat::Celt => "celt",
            SoundFormat::At9 => "at9",
            SoundFormat::Xwma => "xwma",
            SoundFormat::Vorbis => "vorbis",
            SoundFormat::Unknown(v) => return write!(f, "unknown({v})"),
        };
        f.write_str(name)
    }
}

/// Type tag for a per-sample metadata chunk.
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
    /// Decode the on-disk chunk tag.
    pub fn from_u32(value: u32) -> Self {
        match value {
            1 => ChunkType::Channels,
            2 => ChunkType::Frequency,
            3 => ChunkType::Loop,
            6 => ChunkType::XmaSeek,
            7 => ChunkType::DspCoeff,
            10 => ChunkType::XwmaData,
            11 => ChunkType::VorbisData,
            other => ChunkType::Unknown(other),
        }
    }

    /// Encode the chunk tag back to its on-disk value.
    pub fn as_u32(self) -> u32 {
        match self {
            ChunkType::Channels => 1,
            ChunkType::Frequency => 2,
            ChunkType::Loop => 3,
            ChunkType::XmaSeek => 6,
            ChunkType::DspCoeff => 7,
            ChunkType::XwmaData => 10,
            ChunkType::VorbisData => 11,
            ChunkType::Unknown(value) => value,
        }
    }
}

impl fmt::Display for ChunkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkType::Channels => f.write_str("channels"),
            ChunkType::Frequency => f.write_str("frequency"),
            ChunkType::Loop => f.write_str("loop"),
            ChunkType::XmaSeek => f.write_str("xma_seek"),
            ChunkType::DspCoeff => f.write_str("dsp_coeff"),
            ChunkType::XwmaData => f.write_str("xwma_data"),
            ChunkType::VorbisData => f.write_str("vorbis_data"),
            ChunkType::Unknown(v) => write!(f, "unknown({v})"),
        }
    }
}

/// A decoded per-sample metadata chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// Chunk tag.
    pub kind: ChunkType,
    /// Raw chunk payload (length matches the size declared on disk).
    pub data: Vec<u8>,
}

/// Decoded `VORBISDATA` metadata chunk.
///
/// The chunk begins with a CRC32 that keys one of the pre-built Vorbis
/// setup headers, followed by a seek table of `(offset, granule_position)`
/// pairs (see the FSB5 binary template). The table may be sparse — a reference
/// records roughly one entry per second of audio — but each entry's offset
/// points at the start of a packet in the sample data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VorbisData {
    /// Identifier of the setup header required to decode the stream.
    pub setup_crc32: u32,
    /// Seek table entries: byte offset into the sample data plus granule position.
    pub seek_table: Vec<(u32, u32)>,
}

/// One Vorbis packet stored inside an FSB5 `VORBIS` sample.
///
/// Packets are serialised as a little-endian `u16` length followed by exactly
/// that many Vorbis packet bytes; a length of zero terminates the stream.
///
/// There is no separate flags byte: the first bit of the payload is Vorbis'
/// own audio/header discriminator (it is zero for audio packets), so all
/// `size` bytes belong to the packet. This matches both the reference output and
/// the bundled reference decoder (`reference/reference_vorbis.py`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VorbisPacket<'a> {
    /// Packet payload, without the length prefix.
    pub data: &'a [u8],
}

impl<'a> VorbisPacket<'a> {
    /// Whether this is an audio packet.
    ///
    /// Vorbis audio packets begin with a zero bit, so every packet in an FSB5
    /// stream is normally an audio packet.
    pub fn is_audio(&self) -> bool {
        self.data.first().is_some_and(|byte| byte & 0x01 == 0)
    }
}

/// Iterator over the Vorbis packets of an FSB5 sample.
#[derive(Debug, Clone)]
pub struct VorbisPacketIter<'a> {
    data: &'a [u8],
    cursor: usize,
    done: bool,
}

impl<'a> Iterator for VorbisPacketIter<'a> {
    type Item = VorbisPacket<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.cursor + 2 > self.data.len() {
            self.done = true;
            return None;
        }
        let size =
            u16::from_le_bytes([self.data[self.cursor], self.data[self.cursor + 1]]) as usize;
        self.cursor += 2;
        if size == 0 || self.cursor + size > self.data.len() {
            self.done = true;
            return None;
        }
        let payload = &self.data[self.cursor..self.cursor + size];
        self.cursor += size;
        Some(VorbisPacket { data: payload })
    }
}

/// One sample entry in an FSB5 bank.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sample {
    /// Zero-based index inside the bank.
    pub index: usize,
    /// Name from the bank's name table (or `{:04}` of the index when absent).
    pub name: String,
    /// Sample rate in hertz.
    pub frequency: u32,
    /// Channel count.
    pub channels: u32,
    /// Offset of the sample data, in bytes, from the start of the data section.
    pub data_offset: u32,
    /// Number of audio frames.
    pub num_frames: u32,
    /// Loop start frame, when a LOOP chunk is present.
    pub loop_start: Option<u32>,
    /// Loop end frame, when a LOOP chunk is present.
    pub loop_end: Option<u32>,
    /// Whether the sample header carried extra metadata chunks.
    pub has_extra_params: bool,
    /// Decoded metadata chunks in on-disk order.
    pub chunks: Vec<Chunk>,
}

impl Sample {
    /// Duration in seconds, computed from the frame count and sample rate.
    pub fn duration_seconds(&self) -> f32 {
        if self.frequency == 0 {
            0.0
        } else {
            self.num_frames as f32 / self.frequency as f32
        }
    }

    /// Whether the sample carries loop metadata.
    pub fn is_looped(&self) -> bool {
        self.loop_start.is_some() && self.loop_end.is_some()
    }

    /// The `VORBISDATA` chunk, when the sample carries one.
    pub fn vorbis_chunk(&self) -> Option<&Chunk> {
        self.chunks
            .iter()
            .find(|chunk| chunk.kind == ChunkType::VorbisData)
    }

    /// Decode the `VORBISDATA` chunk into its setup-header id and seek table.
    pub fn vorbis_data(&self) -> Option<VorbisData> {
        let payload = &self.vorbis_chunk()?.data;
        if payload.len() < 4 {
            return None;
        }
        let setup_crc32 = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
        let mut seek_table = Vec::new();
        let mut cursor = 4;
        while cursor + 8 <= payload.len() {
            seek_table.push((
                u32::from_le_bytes([
                    payload[cursor],
                    payload[cursor + 1],
                    payload[cursor + 2],
                    payload[cursor + 3],
                ]),
                u32::from_le_bytes([
                    payload[cursor + 4],
                    payload[cursor + 5],
                    payload[cursor + 6],
                    payload[cursor + 7],
                ]),
            ));
            cursor += 8;
        }
        Some(VorbisData {
            setup_crc32,
            seek_table,
        })
    }
}

/// A fully parsed FSB5 bank.
#[derive(Debug, Clone)]
pub struct Fsb5 {
    /// Container version field.
    pub version: u32,
    /// Number of samples declared by the header.
    pub num_samples: usize,
    /// Total size of the sample-header region.
    pub sample_header_size: u32,
    /// Size of the name table.
    pub name_table_size: u32,
    /// Size of the sample-data region.
    pub data_size: u32,
    /// Audio encoding for every sample in the bank.
    pub format: SoundFormat,
    /// Size, in bytes, of the header preceding the sample-header region.
    pub header_size: usize,
    /// Absolute offset of the sample-data region.
    pub data_section_offset: usize,
    /// Decoded samples.
    pub samples: Vec<Sample>,
    /// Raw 8-byte header `zero` field, retained for faithful re-serialisation.
    header_zero: [u8; 8],
    /// Raw 16-byte header `hash` field.
    header_hash: [u8; 16],
    /// Raw 8-byte header `dummy` field.
    header_dummy: [u8; 8],
    /// Complete bank bytes, retained so sample payloads can be sliced on demand.
    data: Vec<u8>,
}

impl Fsb5 {
    /// Parse an FSB5 bank from its raw bytes.
    pub fn parse(data: &[u8]) -> Result<Self, FsbError> {
        if data.len() < 4 {
            return Err(FsbError::TooShort {
                needed: 4,
                got: data.len(),
            });
        }
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&data[0..4]);
        if magic != FSB5_MAGIC {
            return Err(FsbError::BadMagic(magic));
        }

        let version = read_u32(data, 4, "version")?;
        let num_samples = read_u32(data, 8, "numSamples")?;
        let sample_header_size = read_u32(data, 12, "sampleHeaderSize")?;
        let name_table_size = read_u32(data, 16, "nameTableSize")?;
        let data_size = read_u32(data, 20, "dataSize")?;
        let mode = read_u32(data, 24, "mode")?;

        let header_size = if version == 0 {
            FSB5_HEADER_SIZE_V0
        } else {
            FSB5_HEADER_SIZE
        };
        if data.len() < header_size {
            return Err(FsbError::TooShort {
                needed: header_size,
                got: data.len(),
            });
        }

        let mut header_zero = [0u8; 8];
        header_zero.copy_from_slice(&data[28..36]);
        let mut header_hash = [0u8; 16];
        header_hash.copy_from_slice(&data[36..52]);
        let mut header_dummy = [0u8; 8];
        header_dummy.copy_from_slice(&data[52..60]);

        let sample_count = num_samples as usize;
        // Every sample header needs at least its 8-byte fixed part.
        if sample_count.saturating_mul(8) > data.len() {
            return Err(FsbError::SampleCountTooLarge {
                samples: num_samples,
                available_bytes: data.len(),
            });
        }

        let format = SoundFormat::from_u32(mode);

        // ---- sample headers -------------------------------------------------
        let mut cursor = header_size;
        let mut samples = Vec::with_capacity(sample_count);
        for index in 0..sample_count {
            let raw = read_u64(data, cursor, "sample header")?;
            cursor += 8;

            let has_extra_params = bits(raw, 0, 1) == 1;
            let frequency_index = bits(raw, 1, 4) as u32;
            let channels = bits(raw, 5, 1) as u32 + 1;
            let data_offset_units = bits(raw, 6, 28) as u32;
            let num_frames = bits(raw, 34, 30) as u32;

            let mut sample = Sample {
                index,
                name: format!("{index:04}"),
                frequency: frequency_from_index(frequency_index).unwrap_or(0),
                channels,
                data_offset: data_offset_units * 16,
                num_frames,
                has_extra_params,
                ..Default::default()
            };

            if has_extra_params {
                let mut next = true;
                while next {
                    let chunk_raw = read_u32(data, cursor, "metadata chunk")?;
                    cursor += 4;
                    next = bits(chunk_raw as u64, 0, 1) == 1;
                    let size = bits(chunk_raw as u64, 1, 24) as u32;
                    let tag = bits(chunk_raw as u64, 25, 7) as u32;
                    let payload =
                        read_bytes(data, cursor, size as usize, "metadata payload")?.to_vec();
                    cursor += size as usize;
                    apply_chunk(&mut sample, tag, &payload);
                    sample.chunks.push(Chunk {
                        kind: ChunkType::from_u32(tag),
                        data: payload,
                    });
                }
            }

            samples.push(sample);
        }

        // ---- name table -----------------------------------------------------
        let name_table_start = header_size + sample_header_size as usize;
        if name_table_size > 0 && name_table_start <= data.len() {
            let name_table_end = name_table_start.saturating_add(name_table_size as usize);
            for (index, sample) in samples.iter_mut().enumerate() {
                let offset_pos = name_table_start + index * 4;
                let Ok(relative) = read_u32(data, offset_pos, "name offset") else {
                    break;
                };
                let start = name_table_start.saturating_add(relative as usize);
                if start >= data.len() || start >= name_table_end {
                    continue;
                }
                let limit = name_table_end.min(data.len());
                let end = data[start..limit]
                    .iter()
                    .position(|&byte| byte == 0)
                    .map(|position| start + position)
                    .unwrap_or(limit);
                if let Ok(name) = std::str::from_utf8(&data[start..end]) {
                    sample.name = name.to_string();
                }
            }
        }

        // ---- sample data ----------------------------------------------------
        let data_section_offset =
            header_size + sample_header_size as usize + name_table_size as usize;

        Ok(Fsb5 {
            version,
            num_samples: sample_count,
            sample_header_size,
            name_table_size,
            data_size,
            format,
            header_size,
            data_section_offset,
            samples,
            header_zero,
            header_hash,
            header_dummy,
            data: data.to_vec(),
        })
    }

    /// Parse a bank from a file on disk.
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, FsbOpenError> {
        let bytes = std::fs::read(path.as_ref()).map_err(FsbOpenError::Io)?;
        Fsb5::parse(&bytes).map_err(FsbOpenError::Parse)
    }

    /// Number of samples in the bank.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether the bank contains no samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Look up a sample by index.
    pub fn sample(&self, index: usize) -> Option<&Sample> {
        self.samples.get(index)
    }

    /// Extract the raw (still encoded) bytes for a sample.
    ///
    /// For Vorbis/Mpeg banks the bytes are the compressed bitstream; for PCM
    /// banks they are interleaved little-endian samples.
    pub fn sample_data(&self, index: usize) -> Option<&[u8]> {
        let sample = self.samples.get(index)?;
        let start = self.data_section_offset + sample.data_offset as usize;
        let end = self
            .samples
            .get(index + 1)
            .map(|next| self.data_section_offset + next.data_offset as usize)
            .unwrap_or(self.data_section_offset + self.data_size as usize);
        if start > end || end > self.data.len() {
            return None;
        }
        Some(&self.data[start..end])
    }

    /// Iterate the Vorbis packets of a sample in a `VORBIS` bank.
    ///
    /// Returns `None` when the bank is not Vorbis-encoded or the sample has no
    /// data. Decoding the payloads additionally requires the setup header
    /// identified by [`Sample::vorbis_data`], which is stored inside the bank.
    pub fn vorbis_packets(&self, index: usize) -> Option<VorbisPacketIter<'_>> {
        if self.format != SoundFormat::Vorbis {
            return None;
        }
        let data = self.sample_data(index)?;
        Some(VorbisPacketIter {
            data,
            cursor: 0,
            done: false,
        })
    }

    /// Convert a PCM sample into a canonical RIFF/WAVE file.
    ///
    /// Returns `None` for compressed formats (`Vorbis`, `Mpeg`, ADPCM, …),
    /// which require a dedicated decoder. The sample data is written verbatim,
    /// matching the reference layout (little-endian, signed for 16/24/32-bit).
    pub fn sample_to_wav(&self, index: usize) -> Option<Vec<u8>> {
        let sample = self.samples.get(index)?;
        let data = self.sample_data(index)?;
        let (bits, format_tag) = match self.format {
            SoundFormat::Pcm8 => (8u16, 1u16),
            SoundFormat::Pcm16 => (16, 1),
            SoundFormat::Pcm24 => (24, 1),
            SoundFormat::Pcm32 => (32, 1),
            SoundFormat::PcmFloat => (32, 3),
            _ => return None,
        };
        let channels = sample.channels.max(1) as u16;
        let block_align = channels * bits / 8;
        let byte_rate = sample.frequency.saturating_mul(block_align as u32);

        let mut out = Vec::with_capacity(44 + data.len());
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36u32 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&format_tag.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&sample.frequency.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        Some(out)
    }

    /// The complete bank bytes backing this value.
    pub fn raw_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Raw 8-byte `zero` header field, preserved for faithful re-serialisation.
    pub fn header_zero(&self) -> [u8; 8] {
        self.header_zero
    }

    /// Raw 16-byte `hash` header field, preserved for faithful re-serialisation.
    pub fn header_hash(&self) -> [u8; 16] {
        self.header_hash
    }

    /// Raw 8-byte `dummy` header field, preserved for faithful re-serialisation.
    pub fn header_dummy(&self) -> [u8; 8] {
        self.header_dummy
    }

    /// The metadata chunks of a sample that the writer does not derive itself
    /// (everything except `CHANNELS`, `FREQUENCY` and `LOOP`).
    pub fn preserved_chunks(&self, index: usize) -> Vec<(u32, Vec<u8>)> {
        let Some(sample) = self.samples.get(index) else {
            return Vec::new();
        };
        sample
            .chunks
            .iter()
            .filter(|chunk| {
                !matches!(
                    chunk.kind,
                    ChunkType::Channels | ChunkType::Frequency | ChunkType::Loop
                )
            })
            .map(|chunk| (chunk.kind.as_u32(), chunk.data.clone()))
            .collect()
    }
}

/// Convenience error combining I/O and parse failures.
#[derive(Debug)]
pub enum FsbOpenError {
    /// The file could not be read.
    Io(std::io::Error),
    /// The file was read but is not a valid FSB5 bank.
    Parse(FsbError),
}

impl fmt::Display for FsbOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsbOpenError::Io(error) => write!(f, "could not read bank: {error}"),
            FsbOpenError::Parse(error) => write!(f, "could not parse bank: {error}"),
        }
    }
}

impl std::error::Error for FsbOpenError {}

/// Update a sample with a decoded metadata chunk.
fn apply_chunk(sample: &mut Sample, tag: u32, payload: &[u8]) {
    match tag {
        1 if !payload.is_empty() => sample.channels = payload[0] as u32,
        2 if payload.len() >= 4 => {
            sample.frequency = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
        }
        3 if payload.len() >= 8 => {
            sample.loop_start = Some(u32::from_le_bytes([
                payload[0], payload[1], payload[2], payload[3],
            ]));
            sample.loop_end = Some(u32::from_le_bytes([
                payload[4], payload[5], payload[6], payload[7],
            ]));
        }
        _ => {}
    }
}

/// Map a 4-bit frequency index to a sample rate.
pub fn frequency_from_index(index: u32) -> Option<u32> {
    match index {
        1 => Some(8_000),
        2 => Some(11_000),
        3 => Some(11_025),
        4 => Some(16_000),
        5 => Some(22_050),
        6 => Some(24_000),
        7 => Some(32_000),
        8 => Some(44_100),
        9 => Some(48_000),
        _ => None,
    }
}

/// Extract `len` bits starting at `start` from a little-endian bit field.
fn bits(value: u64, start: u32, len: u32) -> u64 {
    let stop = start + len;
    let mask = if stop >= 64 {
        u64::MAX
    } else {
        (1u64 << stop) - 1
    };
    (value & mask) >> start
}

fn read_bytes<'a>(
    data: &'a [u8],
    offset: usize,
    len: usize,
    context: &'static str,
) -> Result<&'a [u8], FsbError> {
    let end = offset
        .checked_add(len)
        .ok_or(FsbError::Truncated { context, offset })?;
    data.get(offset..end)
        .ok_or(FsbError::Truncated { context, offset })
}

fn read_u32(data: &[u8], offset: usize, context: &'static str) -> Result<u32, FsbError> {
    let bytes = read_bytes(data, offset, 4, context)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u64(data: &[u8], offset: usize, context: &'static str) -> Result<u64, FsbError> {
    let bytes = read_bytes(data, offset, 8, context)?;
    let mut array = [0u8; 8];
    array.copy_from_slice(bytes);
    Ok(u64::from_le_bytes(array))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic single-sample PCM16 FSB5 bank.
    fn synthetic_bank() -> Vec<u8> {
        let mut file = Vec::new();
        // header
        file.extend_from_slice(b"FSB5");
        file.extend_from_slice(&1u32.to_le_bytes()); // version
        file.extend_from_slice(&1u32.to_le_bytes()); // numSamples
        file.extend_from_slice(&8u32.to_le_bytes()); // sampleHeaderSize
        file.extend_from_slice(&9u32.to_le_bytes()); // nameTableSize
        file.extend_from_slice(&8u32.to_le_bytes()); // dataSize
        file.extend_from_slice(&2u32.to_le_bytes()); // mode = PCM16
        file.extend_from_slice(&[0u8; 8]); // zero
        file.extend_from_slice(&[0u8; 16]); // hash
        file.extend_from_slice(&[0u8; 8]); // dummy
        assert_eq!(file.len(), FSB5_HEADER_SIZE);

        // sample header: extraParams=0, freq index=8 (44100), channels=1, offset=0, frames=4
        let raw: u64 = (8u64 << 1) | (4u64 << 34);
        file.extend_from_slice(&raw.to_le_bytes());

        // name table: offset[0]=4 then "beep\0"
        file.extend_from_slice(&4u32.to_le_bytes());
        file.extend_from_slice(b"beep\0");

        // data: 4 frames * 1 channel * 2 bytes
        file.extend_from_slice(&[1u8, 2, 3, 4, 5, 6, 7, 8]);
        file
    }

    #[test]
    fn parses_header_and_sample() {
        let bank = Fsb5::parse(&synthetic_bank()).expect("bank parses");
        assert_eq!(bank.version, 1);
        assert_eq!(bank.num_samples, 1);
        assert_eq!(bank.format, SoundFormat::Pcm16);
        assert_eq!(bank.len(), 1);

        let sample = &bank.samples[0];
        assert_eq!(sample.name, "beep");
        assert_eq!(sample.frequency, 44_100);
        assert_eq!(sample.channels, 1);
        assert_eq!(sample.num_frames, 4);
        assert_eq!(sample.data_offset, 0);
        assert!(!sample.is_looped());
    }

    #[test]
    fn extracts_sample_data() {
        let bank = Fsb5::parse(&synthetic_bank()).expect("bank parses");
        let data = bank.sample_data(0).expect("sample data");
        assert_eq!(data, &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn rejects_non_fsb5() {
        let error = Fsb5::parse(b"NOPE....").unwrap_err();
        assert_eq!(error, FsbError::BadMagic(*b"NOPE"));
    }

    #[test]
    fn rejects_short_buffer() {
        assert!(matches!(
            Fsb5::parse(b"FSB"),
            Err(FsbError::TooShort { .. })
        ));
    }

    #[test]
    fn bit_extraction_matches_template() {
        // A 4-bit frequency index of 8 lives in bits [1..5).
        assert_eq!(bits(8u64 << 1, 1, 4), 8);
        // top 30-bit field
        assert_eq!(bits(0xFFFF_FFFF_0000_0000, 34, 30), 0x3FFF_FFFF);
        // low bit
        assert_eq!(bits(1, 0, 1), 1);
        assert_eq!(bits(0, 0, 1), 0);
    }

    #[test]
    fn frequency_index_lookup() {
        assert_eq!(frequency_from_index(8), Some(44_100));
        assert_eq!(frequency_from_index(0), None);
        assert_eq!(frequency_from_index(15), None);
    }

    /// Build a synthetic single-sample VORBIS FSB5 bank with one audio packet.
    fn synthetic_vorbis_bank() -> Vec<u8> {
        let mut file = Vec::new();
        file.extend_from_slice(b"FSB5");
        file.extend_from_slice(&1u32.to_le_bytes()); // version
        file.extend_from_slice(&1u32.to_le_bytes()); // numSamples
                                                     // sampleHeaderSize = 8 (fixed) + 4 (chunk header) + 12 (crc32 + seek entry)
        file.extend_from_slice(&24u32.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes()); // nameTableSize
        file.extend_from_slice(&6u32.to_le_bytes()); // dataSize
        file.extend_from_slice(&15u32.to_le_bytes()); // mode = VORBIS
        file.extend_from_slice(&[0u8; 8]);
        file.extend_from_slice(&[0u8; 16]);
        file.extend_from_slice(&[0u8; 8]);
        assert_eq!(file.len(), FSB5_HEADER_SIZE);

        // sample header: extraParams=1, freq index=8 (44100), channels=1, offset=0
        let raw: u64 = 1 | (8u64 << 1);
        file.extend_from_slice(&raw.to_le_bytes());

        // metadata chunk: next=0, size=12, type=VORBISDATA(11)
        let chunk: u32 = (12u32 << 1) | (11u32 << 25);
        file.extend_from_slice(&chunk.to_le_bytes());
        // VORBISDATA: crc32 + one seek entry (offset, granule)
        file.extend_from_slice(&118_203_318u32.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());

        // data: one packet (size=2, payload 0xAA 0xBB) then terminator
        file.extend_from_slice(&[0x02, 0x00, 0xAA, 0xBB, 0x00, 0x00]);
        file
    }

    #[test]
    fn parses_vorbis_bank_and_packets() {
        let bank = Fsb5::parse(&synthetic_vorbis_bank()).expect("bank parses");
        assert_eq!(bank.format, SoundFormat::Vorbis);
        assert_eq!(bank.format.file_extension(), "ogg");

        let sample = &bank.samples[0];
        assert!(sample.has_extra_params);
        assert_eq!(sample.frequency, 44_100);
        assert_eq!(sample.channels, 1);

        let vorbis = sample.vorbis_data().expect("vorbis data");
        assert_eq!(vorbis.setup_crc32, 118_203_318);
        assert_eq!(vorbis.seek_table, vec![(0, 0)]);

        let packets: Vec<_> = bank.vorbis_packets(0).expect("vorbis packets").collect();
        assert_eq!(packets.len(), 1);
        assert!(packets[0].is_audio());
        assert_eq!(packets[0].data, &[0xAA, 0xBB]);
    }

    #[test]
    fn vorbis_packets_none_for_pcm_bank() {
        let bank = Fsb5::parse(&synthetic_bank()).expect("bank parses");
        assert!(bank.vorbis_packets(0).is_none());
    }

    #[test]
    fn exports_pcm_sample_to_wav() {
        let bank = Fsb5::parse(&synthetic_bank()).expect("bank parses");
        let wav = bank.sample_to_wav(0).expect("wav export");

        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        // audio_format = PCM(1), channels = 1, sample_rate = 44100, bits = 16
        assert_eq!(u16::from_le_bytes([wav[20], wav[21]]), 1);
        assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 1);
        assert_eq!(
            u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]),
            44_100
        );
        assert_eq!(u16::from_le_bytes([wav[34], wav[35]]), 16);
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]), 8);
        assert_eq!(&wav[44..], &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn wav_export_rejects_vorbis() {
        let bank = Fsb5::parse(&synthetic_vorbis_bank()).expect("bank parses");
        assert!(bank.sample_to_wav(0).is_none());
    }
}
