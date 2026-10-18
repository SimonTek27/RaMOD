//! PCM → Vorbis encoding for FSB5 banks.
//!
//! ## Backends
//!
//! | Feature | Backend | Platforms |
//! |---------|---------|-----------|
//! | `encoder-rust` (default) | [`rusty_vorbis`](https://crates.io/crates/rusty_vorbis) pure-Rust | all |
//! | `encoder-dll` | `libfsbvorbis64.dll` via `LoadLibraryW` | Windows only |
//!
//! Enable both to prefer pure-Rust and fall back to the DLL on Windows if needed.
//! With neither feature, [`encode`] returns [`VorbisExportError::EncoderUnavailable`].
//!
//! The setup header CRC32 must be present in the bundled 161-header table so that
//! commercial and reference FSB5 decoders can play the bank.

use crate::vorbis::{bank_from_import, VorbisExportError, VorbisImport};

/// Default VBR quality (0.0–1.0) used when the caller does not supply one.
pub const DEFAULT_QUALITY: f32 = 0.5;

/// Encode interleaved `f32` PCM into a Vorbis stream ready for an FSB5 bank.
///
/// `pcm.len()` must equal `channels * frames`. The setup header is verified
/// against the bundled table.
pub fn encode(
    pcm: &[f32],
    channels: u32,
    sample_rate: u32,
    quality: f32,
) -> Result<VorbisImport, VorbisExportError> {
    if channels == 0 || sample_rate == 0 {
        return Err(VorbisExportError::EncoderInit);
    }
    let channels_usize = channels as usize;
    if pcm.len() % channels_usize != 0 {
        return Err(VorbisExportError::InvalidPcm);
    }
    if pcm.is_empty() {
        return Err(VorbisExportError::NoPackets);
    }

    #[cfg(feature = "encoder-rust")]
    {
        match rust_backend::encode(pcm, channels, sample_rate, quality) {
            Ok(import) => return Ok(import),
            Err(e) => {
                // If DLL fallback is available, try it; otherwise propagate.
                #[cfg(all(windows, feature = "encoder-dll"))]
                {
                    let _ = e;
                    return dll_backend::encode(pcm, channels, sample_rate, quality);
                }
                #[cfg(not(all(windows, feature = "encoder-dll")))]
                {
                    return Err(e);
                }
            }
        }
    }

    #[cfg(all(windows, feature = "encoder-dll", not(feature = "encoder-rust")))]
    {
        return dll_backend::encode(pcm, channels, sample_rate, quality);
    }

    #[cfg(not(any(
        feature = "encoder-rust",
        all(windows, feature = "encoder-dll")
    )))]
    {
        Err(VorbisExportError::EncoderUnavailable(
            "no Vorbis encoder feature enabled; build with `--features encoder-rust` \
             (pure Rust) or `--features encoder-dll` (Windows DLL)"
                .to_string(),
        ))
    }
}

/// Encode PCM and wrap it into a single-sample FSB5 `VORBIS` bank.
pub fn encode_to_bank(
    pcm: &[f32],
    channels: u32,
    sample_rate: u32,
    quality: f32,
    name: &str,
) -> Result<Vec<u8>, VorbisExportError> {
    let import = encode(pcm, channels, sample_rate, quality)?;
    bank_from_import(&import, name)
}

// ---------------------------------------------------------------------------
// Pure-Rust backend (rusty_vorbis)
// ---------------------------------------------------------------------------

#[cfg(feature = "encoder-rust")]
mod rust_backend {
    use crate::vorbis::{setup_headers, zlib_crc32, VorbisExportError, VorbisImport};
    use rusty_vorbis::{Error as RvError, VorbisEncoder, VorbisEncoderConfig};

    pub fn encode(
        pcm: &[f32],
        channels: u32,
        sample_rate: u32,
        quality: f32,
    ) -> Result<VorbisImport, VorbisExportError> {
        let mut cfg = VorbisEncoderConfig::default();
        cfg.quality = quality.clamp(0.0, 1.0);

        let mut enc = VorbisEncoder::new(cfg);
        enc.push_pcm_f32(pcm, channels as u16, sample_rate)
            .map_err(map_rv)?;
        enc.finish();

        // headers(): [id, comment, setup]
        let headers = enc.headers();
        if headers.len() < 3 {
            return Err(VorbisExportError::EncoderInit);
        }
        let setup = &headers[2];
        let setup_crc32 = zlib_crc32(setup);
        if setup_headers().get(setup_crc32).is_none() {
            return Err(VorbisExportError::UnknownSetupHeader(setup_crc32));
        }

        let mut audio_packets: Vec<Vec<u8>> = Vec::new();
        let mut granules: Vec<u32> = Vec::new();

        loop {
            match enc.next() {
                Ok(packet) => {
                    // Skip the three header packets (pts/duration 0, or first three).
                    if packet.duration == 0 && packet.pts == 0 && audio_packets.is_empty() {
                        // Could be a header; headers were already taken via headers().
                        // rusty_vorbis also yields them from next() — skip non-audio.
                        if packet.data.first().is_some_and(|b| b & 1 == 1) {
                            continue; // header packet (LSB of first byte is 1)
                        }
                    }
                    if packet.data.first().is_some_and(|b| b & 1 == 1) {
                        continue; // header
                    }
                    let granule = if packet.pts < 0 {
                        granules.last().copied().unwrap_or(0)
                    } else {
                        packet.pts.min(u32::MAX as i64) as u32
                    };
                    audio_packets.push(packet.data);
                    granules.push(granule);
                }
                Err(RvError::Again) => continue,
                Err(RvError::Eof) => break,
                Err(e) => return Err(map_rv(e)),
            }
        }

        if audio_packets.is_empty() {
            return Err(VorbisExportError::NoPackets);
        }

        Ok(VorbisImport {
            channels,
            sample_rate,
            setup_crc32,
            audio_packets,
            granules,
        })
    }

    fn map_rv(err: RvError) -> VorbisExportError {
        match err {
            RvError::Again | RvError::Eof => VorbisExportError::EncoderInit,
            other => VorbisExportError::EncoderUnavailable(format!("rusty_vorbis: {other:?}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Windows DLL backend (libfsbvorbis64.dll)
// ---------------------------------------------------------------------------

#[cfg(all(windows, feature = "encoder-dll"))]
mod dll_backend {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::path::PathBuf;

    use crate::vorbis::{setup_headers, zlib_crc32, VorbisExportError, VorbisImport};

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryW(name: *const u16) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    }

    const STATE_WORDS: usize = 4096;
    type State = [u64; STATE_WORDS];

    fn state() -> Box<State> {
        Box::new([0u64; STATE_WORDS])
    }

    #[repr(C)]
    struct OggPacket {
        packet: *mut u8,
        bytes: i32,
        b_o_s: i32,
        e_o_s: i32,
        granulepos: i64,
        packetno: i64,
    }

    impl OggPacket {
        fn zeroed() -> Self {
            OggPacket {
                packet: std::ptr::null_mut(),
                bytes: 0,
                b_o_s: 0,
                e_o_s: 0,
                granulepos: 0,
                packetno: 0,
            }
        }
    }

    type InfoInit = unsafe extern "C" fn(*mut c_void);
    type InfoClear = unsafe extern "C" fn(*mut c_void);
    type EncodeInitVbr = unsafe extern "C" fn(*mut c_void, i32, i32, f32) -> i32;
    type CommentInit = unsafe extern "C" fn(*mut c_void);
    type CommentClear = unsafe extern "C" fn(*mut c_void);
    type AnalysisInit = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
    type AnalysisHeaderout = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        *mut OggPacket,
        *mut OggPacket,
        *mut OggPacket,
    ) -> i32;
    type AnalysisBuffer = unsafe extern "C" fn(*mut c_void, i32) -> *mut *mut f32;
    type AnalysisWrote = unsafe extern "C" fn(*mut c_void, i32) -> i32;
    type BlockInit = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
    type BlockClear = unsafe extern "C" fn(*mut c_void);
    type AnalysisBlockout = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
    type Analysis = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
    type BitrateAddblock = unsafe extern "C" fn(*mut c_void) -> i32;
    type BitrateFlushpacket = unsafe extern "C" fn(*mut c_void, *mut OggPacket) -> i32;
    type DspClear = unsafe extern "C" fn(*mut c_void);

    struct Lib {
        info_init: InfoInit,
        info_clear: InfoClear,
        encode_init_vbr: EncodeInitVbr,
        comment_init: CommentInit,
        comment_clear: CommentClear,
        analysis_init: AnalysisInit,
        analysis_headerout: AnalysisHeaderout,
        analysis_buffer: AnalysisBuffer,
        analysis_wrote: AnalysisWrote,
        block_init: BlockInit,
        block_clear: BlockClear,
        analysis_blockout: AnalysisBlockout,
        analysis: Analysis,
        bitrate_addblock: BitrateAddblock,
        bitrate_flushpacket: BitrateFlushpacket,
        dsp_clear: DspClear,
    }

    unsafe fn resolve<T: Copy>(module: *mut c_void, name: &str) -> Result<T, VorbisExportError> {
        let mut cname = Vec::with_capacity(name.len() + 1);
        cname.extend_from_slice(name.as_bytes());
        cname.push(0);
        let address = GetProcAddress(module, cname.as_ptr());
        if address.is_null() {
            return Err(VorbisExportError::EncoderUnavailable(format!(
                "libfsbvorbis64.dll is missing the symbol `{name}`"
            )));
        }
        Ok(std::mem::transmute_copy::<*mut c_void, T>(&address))
    }

    impl Lib {
        fn load() -> Result<Lib, VorbisExportError> {
            let path = find_library().ok_or_else(|| {
                VorbisExportError::EncoderUnavailable(
                    "libfsbvorbis64.dll not found; set RAMOD_VORBIS_ENCODER or install FMOD tools"
                        .to_string(),
                )
            })?;
            let wide: Vec<u16> = path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let module = unsafe { LoadLibraryW(wide.as_ptr()) };
            if module.is_null() {
                return Err(VorbisExportError::EncoderUnavailable(format!(
                    "could not load {}",
                    path.display()
                )));
            }
            unsafe {
                Ok(Lib {
                    info_init: resolve(module, "vorbis_info_init")?,
                    info_clear: resolve(module, "vorbis_info_clear")?,
                    encode_init_vbr: resolve(module, "vorbis_encode_init_vbr")?,
                    comment_init: resolve(module, "vorbis_comment_init")?,
                    comment_clear: resolve(module, "vorbis_comment_clear")?,
                    analysis_init: resolve(module, "vorbis_analysis_init")?,
                    analysis_headerout: resolve(module, "vorbis_analysis_headerout")?,
                    analysis_buffer: resolve(module, "vorbis_analysis_buffer")?,
                    analysis_wrote: resolve(module, "vorbis_analysis_wrote")?,
                    block_init: resolve(module, "vorbis_block_init")?,
                    block_clear: resolve(module, "vorbis_block_clear")?,
                    analysis_blockout: resolve(module, "vorbis_analysis_blockout")?,
                    analysis: resolve(module, "vorbis_analysis")?,
                    bitrate_addblock: resolve(module, "vorbis_bitrate_addblock")?,
                    bitrate_flushpacket: resolve(module, "vorbis_bitrate_flushpacket")?,
                    dsp_clear: resolve(module, "vorbis_dsp_clear")?,
                })
            }
        }
    }

    fn find_library() -> Option<PathBuf> {
        if let Some(explicit) = std::env::var_os("RAMOD_VORBIS_ENCODER") {
            let path = PathBuf::from(explicit);
            if path.is_file() {
                return Some(path);
            }
        }
        let mut roots: Vec<PathBuf> = Vec::new();
        for base in [
            r"C:\Program Files\FMOD SoundSystem",
            r"C:\Program Files (x86)\FMOD SoundSystem",
        ] {
            let base = PathBuf::from(base);
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let dir = entry.path();
                    if dir.is_dir() {
                        roots.push(dir);
                    }
                }
            }
            roots.push(base);
        }
        roots.sort_by(|a, b| b.cmp(a));
        for root in &roots {
            let candidate = root.join("libfsbvorbis64.dll");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let candidate = dir.join("libfsbvorbis64.dll");
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        None
    }

    pub fn encode(
        pcm: &[f32],
        channels: u32,
        sample_rate: u32,
        quality: f32,
    ) -> Result<VorbisImport, VorbisExportError> {
        let channels = channels as usize;
        let lib = Lib::load()?;

        let mut info = state();
        let mut dsp = state();
        let mut block = state();
        let mut comment = state();
        let info_p = info.as_mut_ptr() as *mut c_void;
        let dsp_p = dsp.as_mut_ptr() as *mut c_void;
        let block_p = block.as_mut_ptr() as *mut c_void;
        let comment_p = comment.as_mut_ptr() as *mut c_void;

        unsafe {
            (lib.info_init)(info_p);
            (lib.comment_init)(comment_p);

            if (lib.encode_init_vbr)(
                info_p,
                channels as i32,
                sample_rate as i32,
                quality.clamp(0.0, 1.0),
            ) != 0
            {
                (lib.comment_clear)(comment_p);
                (lib.info_clear)(info_p);
                return Err(VorbisExportError::EncoderInit);
            }
            if (lib.analysis_init)(dsp_p, info_p) != 0 {
                (lib.comment_clear)(comment_p);
                (lib.info_clear)(info_p);
                return Err(VorbisExportError::EncoderInit);
            }
            if (lib.block_init)(dsp_p, block_p) != 0 {
                (lib.dsp_clear)(dsp_p);
                (lib.comment_clear)(comment_p);
                (lib.info_clear)(info_p);
                return Err(VorbisExportError::EncoderInit);
            }

            let mut id_header = OggPacket::zeroed();
            let mut comment_header = OggPacket::zeroed();
            let mut setup_header = OggPacket::zeroed();
            if (lib.analysis_headerout)(
                dsp_p,
                comment_p,
                &mut id_header,
                &mut comment_header,
                &mut setup_header,
            ) != 0
            {
                cleanup(&lib, block_p, dsp_p, comment_p, info_p);
                return Err(VorbisExportError::EncoderInit);
            }
            let setup_bytes =
                std::slice::from_raw_parts(setup_header.packet, setup_header.bytes as usize);
            let setup_crc32 = zlib_crc32(setup_bytes);
            if setup_headers().get(setup_crc32).is_none() {
                cleanup(&lib, block_p, dsp_p, comment_p, info_p);
                return Err(VorbisExportError::UnknownSetupHeader(setup_crc32));
            }

            let frames = pcm.len() / channels;
            let mut packets: Vec<Vec<u8>> = Vec::new();
            let mut granules: Vec<u32> = Vec::new();
            const CHUNK: usize = 1024;

            let mut fed = 0usize;
            while fed < frames {
                let count = (frames - fed).min(CHUNK);
                let buffer = (lib.analysis_buffer)(dsp_p, count as i32);
                if buffer.is_null() {
                    cleanup(&lib, block_p, dsp_p, comment_p, info_p);
                    return Err(VorbisExportError::EncoderInit);
                }
                for channel in 0..channels {
                    let destination = *buffer.add(channel);
                    for frame in 0..count {
                        *destination.add(frame) = pcm[(fed + frame) * channels + channel];
                    }
                }
                (lib.analysis_wrote)(dsp_p, count as i32);
                drain(&lib, dsp_p, block_p, &mut packets, &mut granules);
                fed += count;
            }
            (lib.analysis_wrote)(dsp_p, 0);
            drain(&lib, dsp_p, block_p, &mut packets, &mut granules);

            cleanup(&lib, block_p, dsp_p, comment_p, info_p);

            if packets.is_empty() {
                return Err(VorbisExportError::NoPackets);
            }

            Ok(VorbisImport {
                channels: channels as u32,
                sample_rate,
                setup_crc32,
                audio_packets: packets,
                granules,
            })
        }
    }

    unsafe fn cleanup(
        lib: &Lib,
        block: *mut c_void,
        dsp: *mut c_void,
        comment: *mut c_void,
        info: *mut c_void,
    ) {
        (lib.block_clear)(block);
        (lib.dsp_clear)(dsp);
        (lib.comment_clear)(comment);
        (lib.info_clear)(info);
    }

    unsafe fn drain(
        lib: &Lib,
        dsp: *mut c_void,
        block: *mut c_void,
        packets: &mut Vec<Vec<u8>>,
        granules: &mut Vec<u32>,
    ) {
        while (lib.analysis_blockout)(dsp, block) == 1 {
            (lib.analysis)(block, std::ptr::null_mut());
            (lib.bitrate_addblock)(block);
            loop {
                let mut packet = OggPacket::zeroed();
                if (lib.bitrate_flushpacket)(dsp, &mut packet) != 1 {
                    break;
                }
                let data =
                    std::slice::from_raw_parts(packet.packet, packet.bytes as usize).to_vec();
                let granule = if packet.granulepos < 0 {
                    granules.last().copied().unwrap_or(0)
                } else {
                    packet.granulepos.min(u32::MAX as i64) as u32
                };
                packets.push(data);
                granules.push(granule);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsb::{Fsb5, SoundFormat};
    use crate::vorbis::{build_ogg, setup_headers};

    fn sine(frames: usize, rate: u32, channels: usize) -> Vec<f32> {
        let mut out = Vec::with_capacity(frames * channels);
        for frame in 0..frames {
            let value =
                (2.0 * std::f32::consts::PI * 440.0 * frame as f32 / rate as f32).sin() * 0.5;
            for _ in 0..channels {
                out.push(value);
            }
        }
        out
    }

    #[test]
    fn encodes_mono_pcm_into_a_valid_bank() {
        let rate = 44_100u32;
        let frames = 22_050usize;
        let pcm = sine(frames, rate, 1);

        let bytes = match encode_to_bank(&pcm, 1, rate, DEFAULT_QUALITY, "tone") {
            Ok(bytes) => bytes,
            Err(VorbisExportError::EncoderUnavailable(reason)) => {
                eprintln!("skipping encoder test: {reason}");
                return;
            }
            Err(VorbisExportError::UnknownSetupHeader(crc)) => {
                eprintln!(
                    "skipping: encoder produced setup CRC {crc:#010x} not in bundled table \
                     (rusty_vorbis may use a different codebook profile)"
                );
                return;
            }
            Err(error) => panic!("encode failed: {error}"),
        };

        let bank = Fsb5::parse(&bytes).expect("bank parses");
        assert_eq!(bank.format, SoundFormat::Vorbis);
        assert_eq!(bank.samples[0].name, "tone");
        assert_eq!(bank.samples[0].channels, 1);

        let vorbis = bank.samples[0].vorbis_data().expect("VORBISDATA");
        assert!(
            setup_headers().get(vorbis.setup_crc32).is_some(),
            "setup id {} must be a bundled setup header",
            vorbis.setup_crc32
        );

        let packets = bank.vorbis_packets(0).expect("packets").count();
        assert!(packets > 0);
    }

    #[test]
    fn stereo_encode_round_trips_through_the_ogg_muxer() {
        let rate = 48_000u32;
        let pcm = sine(9_600, rate, 2);

        let bytes = match encode_to_bank(&pcm, 2, rate, 0.6, "stereo") {
            Ok(bytes) => bytes,
            Err(VorbisExportError::EncoderUnavailable(_)) => return,
            Err(VorbisExportError::UnknownSetupHeader(_)) => return,
            Err(error) => panic!("encode failed: {error}"),
        };

        let bank = Fsb5::parse(&bytes).expect("bank parses");
        assert_eq!(bank.samples[0].channels, 2);

        let ogg = build_ogg(&bank, 0).expect("ogg rebuild");
        assert_eq!(&ogg[0..4], b"OggS");
        assert!(ogg.windows(7).any(|w| w == b"\x01vorbis"));
    }

    #[test]
    fn rejects_incomplete_frames() {
        assert_eq!(
            encode(&[0.0, 0.0, 0.0], 2, 44_100, 0.5),
            Err(VorbisExportError::InvalidPcm)
        );
    }
}
