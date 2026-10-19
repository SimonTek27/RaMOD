//! # RaMOD — Rust Audio mod
//!
//! An open-source **middleware for FSB5 sound banks**.
//!
//! ## Modules
//!
//! * [`fsb`] — FSB5 container parsing
//! * [`fsb_write`] — FSB5 writing, [`BankEditor`], WAV reader
//! * [`vorbis`] — setup-header table, Ogg mux/demux, FSB ↔ Ogg
//! * [`vorbis_encode`] — PCM → Vorbis (`encoder-rust` / `encoder-dll`)
//! * [`vorbis_decode`] — Vorbis → PCM (`decoder-rust` / lewton)
//!
//! Default features enable pure-Rust encode + decode (no proprietary DLLs).

pub mod fsb;
pub mod fsb_write;
pub mod vorbis;
pub mod vorbis_encode;

#[cfg(feature = "decoder-rust")]
pub mod vorbis_decode;

pub use fsb::{
    Chunk, ChunkType, Fsb5, FsbError, FsbOpenError, Sample, SoundFormat, VorbisData, VorbisPacket,
};
pub use fsb_write::{BankEditor, EditSample, Fsb5Builder, FsbWriteError, PcmSample, Wav, WavError};
pub use vorbis::{
    bank_from_import, build_ogg, build_ogg_with_vendor, import_ogg, import_ogg_to_bank,
    setup_headers, VorbisExportError, VorbisImport,
};
pub use vorbis_encode::{encode, encode_to_bank, DEFAULT_QUALITY};

#[cfg(feature = "decoder-rust")]
pub use vorbis_decode::{decode_ogg_to_pcm, decode_sample_to_pcm, DecodedPcm, VorbisDecodeError};
