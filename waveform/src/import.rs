//! Audio import — WAV / MP3 / OGG Vorbis / FLAC via Symphonia.

use std::fs::File;
use std::path::Path;

use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::{WaveBuffer, WaveError};

#[derive(Debug)]
pub enum ImportError {
    Io(std::io::Error),
    Format(String),
    Decode(String),
    Wave(WaveError),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Io(e) => write!(f, "io: {e}"),
            ImportError::Format(m) => write!(f, "format: {m}"),
            ImportError::Decode(m) => write!(f, "decode: {m}"),
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

/// Extensions accepted by the editor / DnD.
pub fn is_supported_extension(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "wav" | "wave" | "mp3" | "ogg" | "oga" | "flac"
    )
}

/// Load any supported audio file into a [`WaveBuffer`] (interleaved f32).
pub fn read_audio(path: &Path) -> Result<WaveBuffer, ImportError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| ImportError::Format(e.to_string()))?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| ImportError::Format("no decodable audio track".into()))?
        .clone();

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| ImportError::Format("unknown sample rate".into()))?;
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| ImportError::Decode(e.to_string()))?;

    let mut interleaved: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(SymphoniaError::IoError(e)) => return Err(ImportError::Io(e)),
            Err(e) => {
                // End of stream variants
                let msg = e.to_string();
                if msg.contains("end of stream") || msg.contains("eof") {
                    break;
                }
                return Err(ImportError::Decode(msg));
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(ImportError::Decode(e.to_string())),
        };

        append_decoded(&mut interleaved, decoded, channels);
    }

    if interleaved.is_empty() {
        return Err(ImportError::Decode("no samples decoded".into()));
    }

    // Ensure channel multiple
    let ch = channels.max(1) as usize;
    let n = interleaved.len() - (interleaved.len() % ch);
    interleaved.truncate(n);

    WaveBuffer::new(interleaved, channels.max(1), sample_rate).map_err(ImportError::Wave)
}

/// Back-compat alias.
pub fn read_wav(path: &Path) -> Result<WaveBuffer, ImportError> {
    read_audio(path)
}

fn append_decoded(out: &mut Vec<f32>, decoded: AudioBufferRef<'_>, channels: u16) {
    let ch = channels.max(1) as usize;
    // Every branch feeds the same concrete helper, so no `Sample` bound is needed.
    match decoded {
        AudioBufferRef::F32(buf) => {
            let (frames, planes) = (buf.frames(), buf.spec().channels.count());
            push_frames(out, frames, planes, ch, |c, f| buf.chan(c)[f]);
        }
        AudioBufferRef::U8(buf) => {
            let (frames, planes) = (buf.frames(), buf.spec().channels.count());
            push_frames(out, frames, planes, ch, |c, f| {
                (buf.chan(c)[f] as f32 - 128.0) / 128.0
            });
        }
        AudioBufferRef::U16(buf) => {
            let (frames, planes) = (buf.frames(), buf.spec().channels.count());
            push_frames(out, frames, planes, ch, |c, f| {
                (buf.chan(c)[f] as f32 / 65535.0) * 2.0 - 1.0
            });
        }
        AudioBufferRef::U24(buf) => {
            let (frames, planes) = (buf.frames(), buf.spec().channels.count());
            push_frames(out, frames, planes, ch, |c, f| {
                buf.chan(c)[f].inner() as f32 / 8_388_607.0
            });
        }
        AudioBufferRef::U32(buf) => {
            let (frames, planes) = (buf.frames(), buf.spec().channels.count());
            push_frames(out, frames, planes, ch, |c, f| {
                (buf.chan(c)[f] as f32 / 4_294_967_295.0) * 2.0 - 1.0
            });
        }
        AudioBufferRef::S8(buf) => {
            let (frames, planes) = (buf.frames(), buf.spec().channels.count());
            push_frames(out, frames, planes, ch, |c, f| {
                buf.chan(c)[f] as f32 / 128.0
            });
        }
        AudioBufferRef::S16(buf) => {
            let (frames, planes) = (buf.frames(), buf.spec().channels.count());
            push_frames(out, frames, planes, ch, |c, f| {
                buf.chan(c)[f] as f32 / 32768.0
            });
        }
        AudioBufferRef::S24(buf) => {
            let (frames, planes) = (buf.frames(), buf.spec().channels.count());
            push_frames(out, frames, planes, ch, |c, f| {
                buf.chan(c)[f].inner() as f32 / 8_388_608.0
            });
        }
        AudioBufferRef::S32(buf) => {
            let (frames, planes) = (buf.frames(), buf.spec().channels.count());
            push_frames(out, frames, planes, ch, |c, f| {
                buf.chan(c)[f] as f32 / 2_147_483_648.0
            });
        }
        AudioBufferRef::F64(buf) => {
            let (frames, planes) = (buf.frames(), buf.spec().channels.count());
            push_frames(out, frames, planes, ch, |c, f| buf.chan(c)[f] as f32);
        }
    }
}

fn push_frames(
    out: &mut Vec<f32>,
    frames: usize,
    planes: usize,
    ch: usize,
    mut get: impl FnMut(usize, usize) -> f32,
) {
    for f in 0..frames {
        for c in 0..ch.min(planes) {
            out.push(get(c, f));
        }
        for _ in planes..ch {
            out.push(0.0);
        }
    }
}
