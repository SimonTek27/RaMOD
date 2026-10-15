//! # RaMOD — Rust Audio mod
//!
//! An open-source, dependency-light **middleware for FMOD FSB5 sound banks**,
//! derived from REA reverse engineering of the codec shipped with FMOD Studio
//! 2.03.13.
//!
//! ## Modules
//!
//! * [`fsb`] — FSB5 container parsing: header, per-sample headers, metadata
//!   chunks (`CHANNELS`, `FREQUENCY`, `LOOP`, `VORBISDATA`, …), name table and
//!   sample payloads.
//! * [`fsb_write`] — FSB5 writing: [`Fsb5Builder`] for new banks, the in-place
//!   [`BankEditor`], and the RIFF/WAVE reader.
//! * [`vorbis`] — Vorbis ↔ Ogg: the bundled setup-header table, the Ogg
//!   muxer/demuxer, FSB → Ogg export and Ogg → FSB import.
//! * [`vorbis_encode`] — PCM → Vorbis encoding driven through FMOD's own
//!   `libfsbvorbis64.dll` encoder, loaded at runtime via `GetProcAddress`, so
//!   the middleware needs no Vorbis crate.
//!
//! The crate depends only on the Rust standard library. Decoding, parsing and
//! writing are fully portable; the Vorbis *encoder* additionally needs FMOD's
//! `libfsbvorbis64.dll` at runtime (see [`vorbis_encode`]).
//!
//! ```
//! use ramod::{Fsb5, SoundFormat};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Parse a bank straight from memory.
//! # let bank_bytes: Vec<u8> = Vec::new();
//! # if !bank_bytes.is_empty() {
//! let bank = Fsb5::parse(&bank_bytes)?;
//! println!("{} samples ({:?})", bank.len(), bank.format);
//! # }
//! # Ok(())
//! # }
//! ```

pub mod fsb;
pub mod fsb_write;
pub mod vorbis;
pub mod vorbis_encode;

pub use fsb::{
    Chunk, ChunkType, Fsb5, FsbError, FsbOpenError, Sample, SoundFormat, VorbisData, VorbisPacket,
};
pub use fsb_write::{BankEditor, EditSample, Fsb5Builder, FsbWriteError, PcmSample, Wav, WavError};
pub use vorbis::{
    bank_from_import, build_ogg, build_ogg_with_vendor, import_ogg, import_ogg_to_bank,
    setup_headers, VorbisExportError, VorbisImport,
};
pub use vorbis_encode::{encode, encode_to_bank, DEFAULT_QUALITY};
