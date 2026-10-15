//! Build (write) FSB5 sound banks, and read the WAV files used as input.
//!
//! This is the write side of the audio editor: it takes decoded PCM samples and
//! re-packs them into the container layout implemented by [`crate::fsb`]. The
//! output round-trips through the parser, which is covered by tests.
//!
//! Layout produced (version 1 header, see [`crate::fsb`] for the read side):
//!
//! ```text
//! header (60 bytes)
//! sample headers   (packed 64-bit fields + optional metadata chunks)
//! name table       (u32 offsets + NUL-terminated names)
//! data section     (samples, each 32-byte aligned; `dataOffset` counts 16-byte
//!                   units and the section itself starts 32-byte aligned)
//! ```

use crate::fsb::{Fsb5, FsbError, SoundFormat, FSB5_HEADER_SIZE};

/// Errors raised while building a bank or reading WAV input.
#[derive(Debug)]
pub enum FsbWriteError {
    /// A sample's PCM data length does not fit its channel/format geometry.
    MisalignedSampleData {
        name: String,
        bytes: usize,
        channels: u32,
        bytes_per_sample: usize,
    },
    /// The bank format is not an uncompressed format this writer supports.
    UnsupportedFormat(SoundFormat),
    /// No samples were added.
    NoSamples,
    /// A sample declares more than 255 channels.
    TooManyChannels { name: String, channels: u32 },
    /// Chunk of the WAV input.
    Wav(WavError),
    /// Parse check of the freshly built bank failed (writer bug).
    Verify(FsbError),
}

impl std::fmt::Display for FsbWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsbWriteError::MisalignedSampleData {
                name,
                bytes,
                channels,
                bytes_per_sample,
            } => write!(
                f,
                "sample '{name}' has {bytes} bytes, not a multiple of {channels} channels x {bytes_per_sample} bytes"
            ),
            FsbWriteError::UnsupportedFormat(format) => {
                write!(f, "the FSB5 writer cannot encode {format} banks")
            }
            FsbWriteError::NoSamples => f.write_str("a bank needs at least one sample"),
            FsbWriteError::TooManyChannels { name, channels } => {
                write!(f, "sample '{name}' has {channels} channels (max 255)")
            }
            FsbWriteError::Wav(error) => write!(f, "WAV input: {error}"),
            FsbWriteError::Verify(error) => write!(f, "built bank failed self-verification: {error}"),
        }
    }
}

impl std::error::Error for FsbWriteError {}

impl From<WavError> for FsbWriteError {
    fn from(error: WavError) -> Self {
        FsbWriteError::Wav(error)
    }
}

/// Errors raised while parsing a WAV file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WavError {
    /// The file is too short or a chunk runs past the end.
    Truncated,
    /// The file does not start with `RIFF`/`WAVE`.
    NotRiffWave,
    /// No `fmt ` chunk was found.
    MissingFormat,
    /// No `data` chunk was found.
    MissingData,
    /// An audio format tag this reader does not support.
    UnsupportedAudioFormat(u16),
    /// A bits-per-sample value this reader does not support.
    UnsupportedBitDepth(u16),
}

impl std::fmt::Display for WavError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WavError::Truncated => f.write_str("file is truncated"),
            WavError::NotRiffWave => f.write_str("not a RIFF/WAVE file"),
            WavError::MissingFormat => f.write_str("missing 'fmt ' chunk"),
            WavError::MissingData => f.write_str("missing 'data' chunk"),
            WavError::UnsupportedAudioFormat(tag) => {
                write!(f, "unsupported WAV audio format tag {tag}")
            }
            WavError::UnsupportedBitDepth(bits) => {
                write!(f, "unsupported WAV bit depth {bits}")
            }
        }
    }
}

impl std::error::Error for WavError {}

/// A PCM sample destined for an FSB5 bank.
#[derive(Debug, Clone)]
pub struct PcmSample {
    /// Name stored in the bank's name table.
    pub name: String,
    /// Sample rate in hertz.
    pub frequency: u32,
    /// Channel count.
    pub channels: u32,
    /// Loop start frame, when the sample should loop.
    pub loop_start: Option<u32>,
    /// Loop end frame, when the sample should loop.
    pub loop_end: Option<u32>,
    /// Interleaved little-endian PCM payload.
    pub data: Vec<u8>,
}

impl PcmSample {
    /// Create a sample from interleaved PCM data.
    pub fn new(name: impl Into<String>, frequency: u32, channels: u32, data: Vec<u8>) -> Self {
        PcmSample {
            name: name.into(),
            frequency,
            channels,
            loop_start: None,
            loop_end: None,
            data,
        }
    }

    /// Attach a loop region (in frames).
    pub fn with_loop(mut self, start: u32, end: u32) -> Self {
        self.loop_start = Some(start);
        self.loop_end = Some(end);
        self
    }
}

/// Builder that serialises PCM samples into an FSB5 bank.
#[derive(Debug, Clone)]
pub struct Fsb5Builder {
    format: SoundFormat,
    samples: Vec<PcmSample>,
}

impl Fsb5Builder {
    /// Start a bank with the given (uncompressed) format.
    pub fn new(format: SoundFormat) -> Self {
        Fsb5Builder {
            format,
            samples: Vec::new(),
        }
    }

    /// Append a sample.
    pub fn push(&mut self, sample: PcmSample) -> &mut Self {
        self.samples.push(sample);
        self
    }

    /// Append a sample and return `self` (builder style).
    pub fn with_sample(mut self, sample: PcmSample) -> Self {
        self.samples.push(sample);
        self
    }

    /// Number of samples staged so far.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether no samples have been staged.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Serialise the bank.
    ///
    /// The produced bytes are parsed back and verified before being returned,
    /// so a successful build is guaranteed to be readable by [`crate::fsb`].
    pub fn build(&self) -> Result<Vec<u8>, FsbWriteError> {
        if self.samples.is_empty() {
            return Err(FsbWriteError::NoSamples);
        }
        let bytes_per_sample = self
            .format
            .bytes_per_sample()
            .ok_or(FsbWriteError::UnsupportedFormat(self.format))?;

        let mut records = Vec::with_capacity(self.samples.len());
        for sample in &self.samples {
            let channels = sample.channels.max(1);
            if channels > 255 {
                return Err(FsbWriteError::TooManyChannels {
                    name: sample.name.clone(),
                    channels,
                });
            }
            let frame_bytes = channels as usize * bytes_per_sample;
            if frame_bytes == 0 || sample.data.len() % frame_bytes != 0 {
                return Err(FsbWriteError::MisalignedSampleData {
                    name: sample.name.clone(),
                    bytes: sample.data.len(),
                    channels,
                    bytes_per_sample,
                });
            }
            records.push(SampleRecord {
                name: sample.name.clone(),
                frequency: sample.frequency,
                channels,
                num_frames: (sample.data.len() / frame_bytes) as u32,
                loop_start: sample.loop_start,
                loop_end: sample.loop_end,
                data: sample.data.clone(),
                extra_chunks: Vec::new(),
            });
        }

        serialize_bank(&BankRecord {
            version: 1,
            format: self.format,
            zero: [0u8; 8],
            hash: [0u8; 16],
            dummy: [0u8; 8],
            samples: records,
        })
    }
}

/// One sample, ready to be serialised.
#[derive(Debug, Clone)]
struct SampleRecord {
    name: String,
    frequency: u32,
    channels: u32,
    num_frames: u32,
    loop_start: Option<u32>,
    loop_end: Option<u32>,
    data: Vec<u8>,
    /// Metadata chunks emitted verbatim (everything the writer does not derive).
    extra_chunks: Vec<(u32, Vec<u8>)>,
}

/// A whole bank, ready to be serialised.
#[derive(Debug, Clone)]
struct BankRecord {
    version: u32,
    format: SoundFormat,
    zero: [u8; 8],
    hash: [u8; 16],
    dummy: [u8; 8],
    samples: Vec<SampleRecord>,
}

/// Serialise a bank record, verifying the result parses back.
fn serialize_bank(record: &BankRecord) -> Result<Vec<u8>, FsbWriteError> {
    // ---- data section (each sample 32-byte aligned) -------------------------
    // FMOD aligns every sample to a 32-byte boundary and pads the section to the
    // same boundary; `dataOffset` is stored in 16-byte units, so a 32-byte
    // boundary is always an even unit count.
    const DATA_ALIGN: usize = 32;
    let mut data_blob = Vec::new();
    let mut data_offsets = Vec::with_capacity(record.samples.len());
    for sample in &record.samples {
        while data_blob.len() % DATA_ALIGN != 0 {
            data_blob.push(0);
        }
        data_offsets.push((data_blob.len() / 16) as u32);
        data_blob.extend_from_slice(&sample.data);
    }
    while data_blob.len() % DATA_ALIGN != 0 {
        data_blob.push(0);
    }
    let data_size = data_blob.len() as u32;

    // ---- sample headers -----------------------------------------------------
    let mut sample_headers = Vec::new();
    for (sample, &offset_units) in record.samples.iter().zip(data_offsets.iter()) {
        let channels = sample.channels.max(1);
        let mut chunks: Vec<(u32, Vec<u8>)> = Vec::new();
        let frequency_index = frequency_index(sample.frequency);
        if frequency_index.is_none() {
            chunks.push((2, sample.frequency.to_le_bytes().to_vec())); // FREQUENCY
        }
        if channels != 1 && channels != 2 {
            chunks.push((1, vec![channels as u8])); // CHANNELS
        }
        if let (Some(start), Some(end)) = (sample.loop_start, sample.loop_end) {
            let mut payload = Vec::with_capacity(8);
            payload.extend_from_slice(&start.to_le_bytes());
            payload.extend_from_slice(&end.to_le_bytes());
            chunks.push((3, payload)); // LOOP
        }
        chunks.extend(sample.extra_chunks.iter().cloned());

        let has_chunks = !chunks.is_empty();
        let two_channels = if channels == 2 { 1u64 } else { 0 };
        let frequency_field = frequency_index.unwrap_or(0) as u64;
        let packed = (has_chunks as u64)
            | (frequency_field << 1)
            | (two_channels << 5)
            | ((offset_units as u64 & 0x0FFF_FFFF) << 6)
            | ((sample.num_frames as u64 & 0x3FFF_FFFF) << 34);
        sample_headers.extend_from_slice(&packed.to_le_bytes());

        for (chunk_index, (tag, payload)) in chunks.iter().enumerate() {
            let next = (chunk_index + 1 < chunks.len()) as u32;
            let size = payload.len() as u32;
            let header = next | (size << 1) | (tag << 25);
            sample_headers.extend_from_slice(&header.to_le_bytes());
            sample_headers.extend_from_slice(payload);
        }
    }
    let sample_header_size = sample_headers.len() as u32;

    // ---- name table ---------------------------------------------------------
    let count = record.samples.len();
    let mut names_blob = Vec::new();
    let mut name_offsets = Vec::with_capacity(count);
    for sample in &record.samples {
        name_offsets.push((4 * count + names_blob.len()) as u32);
        names_blob.extend_from_slice(sample.name.as_bytes());
        names_blob.push(0);
    }
    // FMOD pads the name table so the data section starts on a 32-byte boundary
    // (`60 + sampleHeaderSize + nameTableSize` is a multiple of 32 in every
    // FMOD-produced bank); the pad bytes live at the end of the name table.
    let mut name_table_size = (4 * count + names_blob.len()) as u32;
    while (FSB5_HEADER_SIZE as u32 + sample_header_size + name_table_size) % 32 != 0 {
        name_table_size += 1;
        names_blob.push(0);
    }

    // ---- assemble -----------------------------------------------------------
    let mut out =
        Vec::with_capacity(60 + sample_headers.len() + name_table_size as usize + data_blob.len());
    out.extend_from_slice(b"FSB5");
    out.extend_from_slice(&record.version.to_le_bytes());
    out.extend_from_slice(&(count as u32).to_le_bytes());
    out.extend_from_slice(&sample_header_size.to_le_bytes());
    out.extend_from_slice(&name_table_size.to_le_bytes());
    out.extend_from_slice(&data_size.to_le_bytes());
    out.extend_from_slice(&record.format.as_u32().to_le_bytes());
    out.extend_from_slice(&record.zero);
    out.extend_from_slice(&record.hash);
    out.extend_from_slice(&record.dummy);
    out.extend_from_slice(&sample_headers);
    for offset in &name_offsets {
        out.extend_from_slice(&offset.to_le_bytes());
    }
    out.extend_from_slice(&names_blob);
    out.extend_from_slice(&data_blob);

    // Self-verify: the bank must parse back.
    crate::fsb::Fsb5::parse(&out).map_err(FsbWriteError::Verify)?;
    Ok(out)
}

/// A mutable view over a parsed bank, allowing samples to be edited and the
/// bank re-serialised while preserving everything the editor does not touch
/// (codec metadata chunks, header fields, codec payloads).
///
/// ```no_run
/// # use ramod::{BankEditor, Fsb5};
/// # let bank = Fsb5::parse(&[0u8; 0]).unwrap();
/// let mut editor = BankEditor::from_bank(&bank);
/// editor.set_loop(0, 0, 44_100);
/// let rebuilt = editor.build().unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct BankEditor {
    version: u32,
    format: SoundFormat,
    zero: [u8; 8],
    hash: [u8; 16],
    dummy: [u8; 8],
    samples: Vec<EditSample>,
}

/// One editable sample inside a [`BankEditor`].
#[derive(Debug, Clone)]
pub struct EditSample {
    /// Name stored in the name table.
    pub name: String,
    /// Sample rate in hertz.
    pub frequency: u32,
    /// Channel count.
    pub channels: u32,
    /// Number of audio frames (`samples` field).
    pub num_frames: u32,
    /// Loop start frame, when the sample loops.
    pub loop_start: Option<u32>,
    /// Loop end frame, when the sample loops.
    pub loop_end: Option<u32>,
    /// Encoded (or PCM) payload, trimmed of inter-sample alignment padding.
    pub data: Vec<u8>,
    /// Metadata chunks preserved verbatim.
    extra_chunks: Vec<(u32, Vec<u8>)>,
}

impl EditSample {
    /// Create a sample from its essential fields, with no metadata chunks.
    pub fn new(
        name: impl Into<String>,
        frequency: u32,
        channels: u32,
        num_frames: u32,
        data: Vec<u8>,
    ) -> Self {
        EditSample {
            name: name.into(),
            frequency,
            channels: channels.max(1),
            num_frames,
            loop_start: None,
            loop_end: None,
            data,
            extra_chunks: Vec::new(),
        }
    }

    /// Attach a metadata chunk (tag + payload), e.g. `VORBISDATA` (11).
    pub fn with_chunk(mut self, tag: u32, payload: Vec<u8>) -> Self {
        self.extra_chunks.push((tag, payload));
        self
    }

    /// Set a loop region (in frames).
    pub fn with_loop(mut self, start: u32, end: u32) -> Self {
        self.loop_start = Some(start);
        self.loop_end = Some(end);
        self
    }

    /// The metadata chunks carried by this sample (tag + payload).
    pub fn chunks(&self) -> &[(u32, Vec<u8>)] {
        &self.extra_chunks
    }
}

impl BankEditor {
    /// Start a new, empty bank for `format` (header fields zeroed, version 1).
    pub fn new(format: SoundFormat) -> Self {
        BankEditor {
            version: 1,
            format,
            zero: [0u8; 8],
            hash: [0u8; 16],
            dummy: [0u8; 8],
            samples: Vec::new(),
        }
    }

    /// Append a sample.
    pub fn push(&mut self, sample: EditSample) -> &mut Self {
        self.samples.push(sample);
        self
    }

    /// Start editing a parsed bank.
    pub fn from_bank(bank: &Fsb5) -> Self {
        let samples = bank
            .samples
            .iter()
            .enumerate()
            .map(|(index, sample)| {
                let raw = bank.sample_data(index).unwrap_or(&[]);
                let length = effective_length(bank, index, raw);
                EditSample {
                    name: sample.name.clone(),
                    frequency: sample.frequency,
                    channels: sample.channels,
                    num_frames: sample.num_frames,
                    loop_start: sample.loop_start,
                    loop_end: sample.loop_end,
                    data: raw[..length].to_vec(),
                    extra_chunks: bank.preserved_chunks(index),
                }
            })
            .collect();

        BankEditor {
            version: bank.version,
            format: bank.format,
            zero: bank.header_zero(),
            hash: bank.header_hash(),
            dummy: bank.header_dummy(),
            samples,
        }
    }

    /// The bank's audio format.
    pub fn format(&self) -> SoundFormat {
        self.format
    }

    /// Number of samples.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether the bank has no samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Immutable access to a sample.
    pub fn sample(&self, index: usize) -> Option<&EditSample> {
        self.samples.get(index)
    }

    /// Mutable access to a sample.
    pub fn sample_mut(&mut self, index: usize) -> Option<&mut EditSample> {
        self.samples.get_mut(index)
    }

    /// All samples.
    pub fn samples(&self) -> &[EditSample] {
        &self.samples
    }

    /// Rename a sample. Returns `false` if the index is out of range.
    pub fn rename(&mut self, index: usize, name: impl Into<String>) -> bool {
        match self.samples.get_mut(index) {
            Some(sample) => {
                sample.name = name.into();
                true
            }
            None => false,
        }
    }

    /// Change a sample's rate. Returns `false` if the index is out of range.
    pub fn set_frequency(&mut self, index: usize, hertz: u32) -> bool {
        match self.samples.get_mut(index) {
            Some(sample) => {
                sample.frequency = hertz;
                true
            }
            None => false,
        }
    }

    /// Set a loop region (in frames). Returns `false` if the index is invalid.
    pub fn set_loop(&mut self, index: usize, start: u32, end: u32) -> bool {
        match self.samples.get_mut(index) {
            Some(sample) => {
                sample.loop_start = Some(start);
                sample.loop_end = Some(end);
                true
            }
            None => false,
        }
    }

    /// Remove a sample's loop region. Returns `false` if the index is invalid.
    pub fn clear_loop(&mut self, index: usize) -> bool {
        match self.samples.get_mut(index) {
            Some(sample) => {
                sample.loop_start = None;
                sample.loop_end = None;
                true
            }
            None => false,
        }
    }

    /// Replace a sample's payload.
    ///
    /// For PCM banks the frame count is recomputed from the new data; for other
    /// formats `num_frames` is left for the caller to override via
    /// [`BankEditor::sample_mut`].
    pub fn set_data(&mut self, index: usize, data: Vec<u8>) -> bool {
        let format = self.format;
        match self.samples.get_mut(index) {
            Some(sample) => {
                if let Some(bytes_per_sample) = format.bytes_per_sample() {
                    let frame_bytes = sample.channels.max(1) as usize * bytes_per_sample;
                    // `frame_bytes` is always >= 1, so the division cannot fail.
                    sample.num_frames = data.len().checked_div(frame_bytes).unwrap_or(0) as u32;
                }
                sample.data = data;
                true
            }
            None => false,
        }
    }

    /// Discard the preserved header hash (some tools zero it after edits).
    pub fn clear_hash(&mut self) {
        self.hash = [0u8; 16];
    }

    /// Serialise the edited bank.
    pub fn build(&self) -> Result<Vec<u8>, FsbWriteError> {
        if self.samples.is_empty() {
            return Err(FsbWriteError::NoSamples);
        }
        let records = self
            .samples
            .iter()
            .map(|sample| SampleRecord {
                name: sample.name.clone(),
                frequency: sample.frequency,
                channels: sample.channels.max(1),
                num_frames: sample.num_frames,
                loop_start: sample.loop_start,
                loop_end: sample.loop_end,
                data: sample.data.clone(),
                extra_chunks: sample.extra_chunks.clone(),
            })
            .collect();

        serialize_bank(&BankRecord {
            version: self.version,
            format: self.format,
            zero: self.zero,
            hash: self.hash,
            dummy: self.dummy,
            samples: records,
        })
    }
}

/// Length of the meaningful payload, excluding inter-sample alignment padding.
fn effective_length(bank: &Fsb5, index: usize, data: &[u8]) -> usize {
    let Some(sample) = bank.samples.get(index) else {
        return data.len();
    };
    match bank.format {
        SoundFormat::Vorbis => vorbis_stream_len(data).min(data.len()),
        format => match format.bytes_per_sample() {
            Some(bytes_per_sample) => {
                let frame_bytes = sample.channels.max(1) as usize * bytes_per_sample;
                let length = sample.num_frames as usize * frame_bytes;
                if length > 0 && length <= data.len() {
                    length
                } else {
                    data.len()
                }
            }
            None => data.len(),
        },
    }
}

/// Length of a Vorbis packet stream up to and including its terminating
/// zero-length packet, or the whole buffer when no terminator is present.
fn vorbis_stream_len(data: &[u8]) -> usize {
    let mut cursor = 0usize;
    loop {
        if cursor + 2 > data.len() {
            return data.len();
        }
        let size = u16::from_le_bytes([data[cursor], data[cursor + 1]]) as usize;
        cursor += 2;
        if size == 0 {
            return cursor;
        }
        if cursor + size > data.len() {
            return data.len();
        }
        cursor += size;
    }
}

/// Reverse lookup of the 4-bit frequency index used in sample headers.
pub fn frequency_index(rate: u32) -> Option<u32> {
    match rate {
        8_000 => Some(1),
        11_000 => Some(2),
        11_025 => Some(3),
        16_000 => Some(4),
        22_050 => Some(5),
        24_000 => Some(6),
        32_000 => Some(7),
        44_100 => Some(8),
        48_000 => Some(9),
        _ => None,
    }
}

/// A decoded WAV file.
#[derive(Debug, Clone)]
pub struct Wav {
    /// Audio format tag (`1` = PCM, `3` = IEEE float).
    pub audio_format: u16,
    /// Channel count.
    pub channels: u16,
    /// Sample rate.
    pub sample_rate: u32,
    /// Bits per sample.
    pub bits_per_sample: u16,
    /// Interleaved PCM payload.
    pub data: Vec<u8>,
}

impl Wav {
    /// Parse a RIFF/WAVE file (PCM and IEEE float only).
    pub fn parse(bytes: &[u8]) -> Result<Wav, WavError> {
        if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(WavError::NotRiffWave);
        }

        let mut cursor = 12usize;
        let mut format: Option<(u16, u16, u32, u16)> = None;
        let mut data: Option<Vec<u8>> = None;

        while cursor + 8 <= bytes.len() {
            let id = &bytes[cursor..cursor + 4];
            let size = u32::from_le_bytes([
                bytes[cursor + 4],
                bytes[cursor + 5],
                bytes[cursor + 6],
                bytes[cursor + 7],
            ]) as usize;
            let body = cursor + 8;
            if body + size > bytes.len() {
                break;
            }
            match id {
                b"fmt " => {
                    if size < 16 {
                        return Err(WavError::Truncated);
                    }
                    let audio_format = u16::from_le_bytes([bytes[body], bytes[body + 1]]);
                    let channels = u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]);
                    let sample_rate = u32::from_le_bytes([
                        bytes[body + 4],
                        bytes[body + 5],
                        bytes[body + 6],
                        bytes[body + 7],
                    ]);
                    let bits_per_sample = u16::from_le_bytes([bytes[body + 14], bytes[body + 15]]);
                    format = Some((audio_format, channels, sample_rate, bits_per_sample));
                }
                b"data" => {
                    data = Some(bytes[body..body + size].to_vec());
                }
                _ => {}
            }
            // Chunks are word-aligned.
            cursor = body + size + (size & 1);
        }

        let (audio_format, channels, sample_rate, bits_per_sample) =
            format.ok_or(WavError::MissingFormat)?;
        let data = data.ok_or(WavError::MissingData)?;

        if audio_format != 1 && audio_format != 3 {
            return Err(WavError::UnsupportedAudioFormat(audio_format));
        }
        if !matches!(bits_per_sample, 8 | 16 | 24 | 32) {
            return Err(WavError::UnsupportedBitDepth(bits_per_sample));
        }

        Ok(Wav {
            audio_format,
            channels,
            sample_rate,
            bits_per_sample,
            data,
        })
    }

    /// The FSB5 uncompressed format matching this WAV.
    pub fn sound_format(&self) -> Option<SoundFormat> {
        match (self.audio_format, self.bits_per_sample) {
            (1, 8) => Some(SoundFormat::Pcm8),
            (1, 16) => Some(SoundFormat::Pcm16),
            (1, 24) => Some(SoundFormat::Pcm24),
            (1, 32) => Some(SoundFormat::Pcm32),
            (3, 32) => Some(SoundFormat::PcmFloat),
            _ => None,
        }
    }

    /// Convert into an FSB5-ready PCM sample.
    pub fn into_pcm_sample(self, name: impl Into<String>) -> PcmSample {
        PcmSample::new(name, self.sample_rate, self.channels as u32, self.data)
    }

    /// Decode the payload into interleaved `f32` samples in `[-1.0, 1.0]`.
    ///
    /// Returns an empty vector for formats without a conversion (which cannot
    /// occur for a `Wav` produced by [`Wav::parse`]).
    pub fn to_f32_samples(&self) -> Vec<f32> {
        match (self.audio_format, self.bits_per_sample) {
            (3, 32) => self
                .data
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect(),
            (1, 8) => self
                .data
                .iter()
                .map(|&b| (b as f32 - 128.0) / 128.0)
                .collect(),
            (1, 16) => self
                .data
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32_768.0)
                .collect(),
            (1, 24) => self
                .data
                .chunks_exact(3)
                .map(|b| {
                    let raw = ((b[2] as i32) << 16) | ((b[1] as i32) << 8) | (b[0] as i32);
                    ((raw << 8) >> 8) as f32 / 8_388_608.0
                })
                .collect(),
            (1, 32) => self
                .data
                .chunks_exact(4)
                .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32 / 2_147_483_648.0)
                .collect(),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(frames: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            data.extend_from_slice(&((i as i16) * 100).to_le_bytes());
        }
        data
    }

    #[test]
    fn builds_single_sample_bank_that_round_trips() {
        let sample = PcmSample::new("engine", 44_100, 1, ramp(64));
        let bank = Fsb5Builder::new(SoundFormat::Pcm16)
            .with_sample(sample.clone())
            .build()
            .expect("build");

        let parsed = Fsb5::parse(&bank).expect("parse");
        assert_eq!(parsed.format, SoundFormat::Pcm16);
        assert_eq!(parsed.len(), 1);
        let read = &parsed.samples[0];
        assert_eq!(read.name, "engine");
        assert_eq!(read.frequency, 44_100);
        assert_eq!(read.channels, 1);
        assert_eq!(read.num_frames, 64);
        assert_eq!(parsed.sample_data(0).unwrap(), &sample.data[..]);
    }

    #[test]
    fn builds_multi_sample_bank_with_loops_and_odd_rate() {
        let a = PcmSample::new("a", 44_100, 2, ramp(32));
        let b = PcmSample::new("b", 30_000, 1, ramp(48)).with_loop(4, 40);
        let bank = Fsb5Builder::new(SoundFormat::Pcm16)
            .with_sample(a)
            .with_sample(b)
            .build()
            .expect("build");

        let parsed = Fsb5::parse(&bank).expect("parse");
        assert_eq!(parsed.len(), 2);

        assert_eq!(parsed.samples[0].name, "a");
        assert_eq!(parsed.samples[0].channels, 2);

        let second = &parsed.samples[1];
        assert_eq!(second.name, "b");
        assert_eq!(second.frequency, 30_000); // via FREQUENCY chunk
        assert!(second.is_looped());
        assert_eq!(second.loop_start, Some(4));
        assert_eq!(second.loop_end, Some(40));

        // Every sample is 32-byte aligned (matching FMOD), so the second
        // sample's offset is a multiple of 32 and its payload is preserved at
        // the front.
        assert_eq!(second.data_offset % 32, 0);
        assert_eq!(&parsed.sample_data(1).unwrap()[..96], &ramp(48)[..]);
    }

    #[test]
    fn built_bank_uses_fmod_32_byte_data_layout() {
        // Non-32-byte frame counts force inter-sample padding, exercising alignment.
        let a = PcmSample::new("alpha", 48_000, 2, ramp(6)); // 12 bytes
        let b = PcmSample::new("beta", 44_100, 1, ramp(9));
        let bank = Fsb5Builder::new(SoundFormat::Pcm16)
            .with_sample(a)
            .with_sample(b)
            .build()
            .expect("build");

        let parsed = Fsb5::parse(&bank).expect("parse");

        // FMOD starts the data section on a 32-byte boundary and pads the
        // section to the same boundary; every sample is 32-byte aligned.
        assert_eq!(parsed.data_section_offset % 32, 0);
        assert_eq!(parsed.data_size % 32, 0);
        for sample in &parsed.samples {
            assert_eq!(sample.data_offset % 32, 0);
        }

        // `60 + sampleHeaderSize + nameTableSize` is a multiple of 32: the name
        // table absorbs the padding needed to reach that boundary.
        let unpadded = 4 * parsed.num_samples + "alpha\0beta\0".len();
        assert!(
            parsed.name_table_size as usize >= unpadded,
            "name table must fit the offsets and NUL-terminated names"
        );
        assert_eq!(
            (parsed.header_size
                + parsed.sample_header_size as usize
                + parsed.name_table_size as usize)
                % 32,
            0
        );
    }

    #[test]
    fn rejects_data_that_does_not_fit_geometry() {
        // 2 channels x 2 bytes = 4-byte frames; 7 bytes is invalid.
        let sample = PcmSample::new("bad", 44_100, 2, vec![0u8; 7]);
        let error = Fsb5Builder::new(SoundFormat::Pcm16)
            .with_sample(sample)
            .build()
            .unwrap_err();
        assert!(matches!(error, FsbWriteError::MisalignedSampleData { .. }));
    }

    #[test]
    fn rejects_empty_bank() {
        let error = Fsb5Builder::new(SoundFormat::Pcm16).build().unwrap_err();
        assert!(matches!(error, FsbWriteError::NoSamples));
    }

    fn wav_bytes(audio_format: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36u32 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&audio_format.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        let block_align = channels * bits / 8;
        out.extend_from_slice(&(rate * block_align as u32).to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn parses_pcm16_wav() {
        let data = ramp(16);
        let wav = Wav::parse(&wav_bytes(1, 1, 44_100, 16, &data)).expect("parse wav");
        assert_eq!(wav.channels, 1);
        assert_eq!(wav.sample_rate, 44_100);
        assert_eq!(wav.bits_per_sample, 16);
        assert_eq!(wav.data, data);
        assert_eq!(wav.sound_format(), Some(SoundFormat::Pcm16));
    }

    #[test]
    fn wav_to_bank_round_trip() {
        let wav = Wav::parse(&wav_bytes(1, 2, 48_000, 16, &ramp(64))).expect("parse wav");
        let format = wav.sound_format().unwrap();
        let sample = wav.into_pcm_sample("hit");
        let bank = Fsb5Builder::new(format)
            .with_sample(sample)
            .build()
            .expect("build");

        let parsed = Fsb5::parse(&bank).expect("parse");
        assert_eq!(parsed.samples[0].name, "hit");
        assert_eq!(parsed.samples[0].frequency, 48_000);
        assert_eq!(parsed.samples[0].channels, 2);
    }

    #[test]
    fn rejects_non_wave_input() {
        assert!(matches!(
            Wav::parse(b"NOPE!!!!"),
            Err(WavError::NotRiffWave)
        ));
    }

    // ---- BankEditor ------------------------------------------------------

    /// A minimal single-sample VORBIS bank with one audio packet and a
    /// VORBISDATA chunk (setup id 118203318).
    fn synthetic_vorbis_bank_bytes() -> Vec<u8> {
        let mut file = Vec::new();
        file.extend_from_slice(b"FSB5");
        file.extend_from_slice(&1u32.to_le_bytes()); // version
        file.extend_from_slice(&1u32.to_le_bytes()); // numSamples
        file.extend_from_slice(&24u32.to_le_bytes()); // sampleHeaderSize
        file.extend_from_slice(&0u32.to_le_bytes()); // nameTableSize
        file.extend_from_slice(&6u32.to_le_bytes()); // dataSize
        file.extend_from_slice(&15u32.to_le_bytes()); // VORBIS
        file.extend_from_slice(&[0u8; 8]);
        file.extend_from_slice(&[0u8; 16]);
        file.extend_from_slice(&[0u8; 8]);

        let raw: u64 = 1 | (8u64 << 1); // extraParams, 44100, mono
        file.extend_from_slice(&raw.to_le_bytes());

        let chunk: u32 = (12u32 << 1) | (11u32 << 25); // VORBISDATA, 12 bytes
        file.extend_from_slice(&chunk.to_le_bytes());
        file.extend_from_slice(&118_203_318u32.to_le_bytes()); // setup id
        file.extend_from_slice(&0u32.to_le_bytes()); // seek offset
        file.extend_from_slice(&0u32.to_le_bytes()); // seek granule

        file.extend_from_slice(&[0x02, 0x00, 0xAA, 0xBB, 0x00, 0x00]);
        file
    }

    #[test]
    fn editor_round_trip_preserves_content() {
        let bytes = Fsb5Builder::new(SoundFormat::Pcm16)
            .with_sample(PcmSample::new("a", 44_100, 2, ramp(32)))
            .with_sample(PcmSample::new("b", 24_000, 1, ramp(9))) // odd length
            .build()
            .unwrap();
        let bank = Fsb5::parse(&bytes).unwrap();

        let rebuilt = BankEditor::from_bank(&bank).build().unwrap();
        let parsed = Fsb5::parse(&rebuilt).unwrap();

        assert_eq!(parsed.len(), 2);
        for index in 0..2 {
            let original = &bank.samples[index];
            let edited = &parsed.samples[index];
            assert_eq!(edited.name, original.name);
            assert_eq!(edited.frequency, original.frequency);
            assert_eq!(edited.channels, original.channels);
            assert_eq!(edited.num_frames, original.num_frames);

            let length = edited.num_frames as usize * edited.channels.max(1) as usize * 2;
            assert_eq!(
                &parsed.sample_data(index).unwrap()[..length],
                &bank.sample_data(index).unwrap()[..length]
            );
        }
    }

    #[test]
    fn editor_re_serialisation_is_idempotent() {
        let bytes = Fsb5Builder::new(SoundFormat::Pcm16)
            .with_sample(PcmSample::new("x", 44_100, 1, ramp(9))) // 18 bytes -> padded
            .build()
            .unwrap();
        let first = BankEditor::from_bank(&Fsb5::parse(&bytes).unwrap())
            .build()
            .unwrap();
        let second = BankEditor::from_bank(&Fsb5::parse(&first).unwrap())
            .build()
            .unwrap();
        assert_eq!(first, second, "padding must not accumulate across edits");
    }

    #[test]
    fn editor_edits_are_persisted() {
        let bytes = Fsb5Builder::new(SoundFormat::Pcm16)
            .with_sample(PcmSample::new("old", 44_100, 1, ramp(16)))
            .build()
            .unwrap();
        let bank = Fsb5::parse(&bytes).unwrap();

        let mut editor = BankEditor::from_bank(&bank);
        assert!(editor.rename(0, "engine_idle"));
        assert!(editor.set_frequency(0, 30_000)); // forces a FREQUENCY chunk
        assert!(editor.set_loop(0, 2, 14));
        assert!(!editor.rename(5, "nope")); // out of range

        let parsed = Fsb5::parse(&editor.build().unwrap()).unwrap();
        assert_eq!(parsed.samples[0].name, "engine_idle");
        assert_eq!(parsed.samples[0].frequency, 30_000);
        assert_eq!(parsed.samples[0].loop_start, Some(2));
        assert_eq!(parsed.samples[0].loop_end, Some(14));

        assert!(editor.clear_loop(0));
        let cleared = Fsb5::parse(&editor.build().unwrap()).unwrap();
        assert!(!cleared.samples[0].is_looped());
    }

    #[test]
    fn editor_replaces_sample_data() {
        let bytes = Fsb5Builder::new(SoundFormat::Pcm16)
            .with_sample(PcmSample::new("x", 44_100, 1, ramp(16)))
            .build()
            .unwrap();
        let bank = Fsb5::parse(&bytes).unwrap();
        let mut editor = BankEditor::from_bank(&bank);

        let replacement = ramp(40);
        assert!(editor.set_data(0, replacement.clone()));
        assert_eq!(editor.sample(0).unwrap().num_frames, 40);

        let parsed = Fsb5::parse(&editor.build().unwrap()).unwrap();
        assert_eq!(parsed.samples[0].num_frames, 40);
        assert_eq!(&parsed.sample_data(0).unwrap()[..80], &replacement[..]);
    }

    #[test]
    fn editor_preserves_vorbis_metadata_and_data() {
        let bank = Fsb5::parse(&synthetic_vorbis_bank_bytes()).unwrap();
        let mut editor = BankEditor::from_bank(&bank);
        assert!(editor.rename(0, "music"));
        assert!(editor.set_loop(0, 0, 5_000));

        let parsed = Fsb5::parse(&editor.build().unwrap()).unwrap();
        assert_eq!(parsed.format, SoundFormat::Vorbis);
        assert_eq!(parsed.samples[0].name, "music");
        assert!(parsed.samples[0].is_looped());

        let vorbis = parsed.samples[0]
            .vorbis_data()
            .expect("VORBISDATA preserved");
        assert_eq!(vorbis.setup_crc32, 118_203_318);

        let packets: Vec<_> = parsed.vorbis_packets(0).unwrap().collect();
        assert_eq!(packets.len(), 1);
        assert!(packets[0].is_audio());
        assert_eq!(packets[0].data, &[0xAA, 0xBB]);
    }
}
