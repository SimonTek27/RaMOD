//! RaMOD command-line interface (Phase 0).
//!
//! Subcommands (initial set):
//! - `list`    — list samples in a bank
//! - `info`    — detailed bank / sample info
//! - `extract` — extract one sample to WAV or Ogg
//!
//! Future: `build`, `edit`, `convert` (see ROADMAP.md item 6).

use std::env;
use std::fs;
use std::path::Path;
use std::process;

use ramod::{build_ogg, Fsb5, SoundFormat};

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage();
        process::exit(2);
    }

    let cmd = args.remove(0);
    let code = match cmd.as_str() {
        "list" => cmd_list(&args),
        "info" => cmd_info(&args),
        "extract" => cmd_extract(&args),
        "help" | "-h" | "--help" => {
            print_usage();
            0
        }
        other => {
            eprintln!("unknown command: {other}");
            print_usage();
            2
        }
    };
    process::exit(code);
}

fn print_usage() {
    eprintln!(
        "RaMOD — open-source FSB5 middleware\n\
         \n\
         Usage:\n\
           ramod list <bank.fsb>\n\
           ramod info <bank.fsb> [--index N]\n\
           ramod extract <bank.fsb> --index N --out <file>\n\
         \n\
         See DESIGN.md / ROADMAP.md / ARCHITECTURE.md for the long-term plan."
    );
}

fn load_bank(path: &str) -> Result<Fsb5, String> {
    let data = fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    Fsb5::parse(&data).map_err(|e| format!("parse {path}: {e}"))
}

fn cmd_list(args: &[String]) -> i32 {
    let Some(path) = args.first() else {
        eprintln!("list: missing <bank.fsb>");
        return 2;
    };
    let bank = match load_bank(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };

    println!(
        "{} — {} sample(s), format={:?}",
        path,
        bank.len(),
        bank.format
    );
    for s in &bank.samples {
        let looped = if s.is_looped() { " loop" } else { "" };
        println!(
            "  [{:>4}] {:<32} {:>6} Hz  {} ch  {:>8} frames{looped}",
            s.index, s.name, s.frequency, s.channels, s.num_frames
        );
    }
    0
}

fn cmd_info(args: &[String]) -> i32 {
    let mut path = None;
    let mut index: Option<usize> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--index" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    eprintln!("info: --index needs a value");
                    return 2;
                };
                index = match v.parse() {
                    Ok(n) => Some(n),
                    Err(_) => {
                        eprintln!("info: invalid index {v}");
                        return 2;
                    }
                };
            }
            p if !p.starts_with('-') && path.is_none() => path = Some(p),
            other => {
                eprintln!("info: unexpected argument {other}");
                return 2;
            }
        }
        i += 1;
    }
    let Some(path) = path else {
        eprintln!("info: missing <bank.fsb>");
        return 2;
    };

    let bank = match load_bank(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };

    println!("file:              {path}");
    println!("version:           {}", bank.version);
    println!("samples:           {}", bank.len());
    println!("format:            {:?}", bank.format);
    println!("sample_header_size:{}", bank.sample_header_size);
    println!("name_table_size:   {}", bank.name_table_size);
    println!("data_size:         {}", bank.data_size);

    if let Some(idx) = index {
        let Some(s) = bank.samples.get(idx) else {
            eprintln!("info: index {idx} out of range (0..{})", bank.len());
            return 1;
        };
        println!("--- sample {idx} ---");
        println!("name:       {}", s.name);
        println!("frequency:  {}", s.frequency);
        println!("channels:   {}", s.channels);
        println!("frames:     {}", s.num_frames);
        println!("duration:   {:.3}s", s.duration_seconds());
        println!("data_offset:{}", s.data_offset);
        println!("loop:       {:?}-{:?}", s.loop_start, s.loop_end);
        println!("chunks:     {}", s.chunks.len());
        for c in &s.chunks {
            println!("  - {:?} ({} bytes)", c.kind, c.data.len());
        }
        if let Some(vd) = s.vorbis_data() {
            println!("vorbis setup CRC32: {:#010x}", vd.setup_crc32);
            println!("vorbis seek entries: {}", vd.seek_table.len());
        }
    }
    0
}

fn cmd_extract(args: &[String]) -> i32 {
    let mut path = None;
    let mut index: Option<usize> = None;
    let mut out: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--index" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    eprintln!("extract: --index needs a value");
                    return 2;
                };
                index = match v.parse() {
                    Ok(n) => Some(n),
                    Err(_) => {
                        eprintln!("extract: invalid index {v}");
                        return 2;
                    }
                };
            }
            "--out" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    eprintln!("extract: --out needs a path");
                    return 2;
                };
                out = Some(v.clone());
            }
            p if !p.starts_with('-') && path.is_none() => path = Some(p),
            other => {
                eprintln!("extract: unexpected argument {other}");
                return 2;
            }
        }
        i += 1;
    }

    let (Some(path), Some(idx), Some(out_path)) = (path, index, out) else {
        eprintln!("extract: require <bank.fsb> --index N --out <file>");
        return 2;
    };

    let bank = match load_bank(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    if idx >= bank.len() {
        eprintln!("extract: index {idx} out of range (0..{})", bank.len());
        return 1;
    }

    let out_path = Path::new(&out_path);
    let result = match bank.format {
        SoundFormat::Vorbis => build_ogg(&bank, idx).map(|bytes| (bytes, "ogg")),
        SoundFormat::Pcm8 | SoundFormat::Pcm16 | SoundFormat::Pcm24 | SoundFormat::Pcm32
        | SoundFormat::PcmFloat => {
            // WAV extract via sample payload + simple RIFF header is future work;
            // for now require Vorbis or document limitation.
            eprintln!(
                "extract: PCM → WAV not yet wired in the CLI (use the library API). \
                 Vorbis → Ogg is supported."
            );
            return 1;
        }
        other => {
            eprintln!("extract: unsupported format {other:?}");
            return 1;
        }
    };

    match result {
        Ok((bytes, _kind)) => {
            if let Err(e) = fs::write(out_path, &bytes) {
                eprintln!("extract: write {}: {e}", out_path.display());
                return 1;
            }
            println!(
                "wrote {} bytes → {}",
                bytes.len(),
                out_path.display()
            );
            0
        }
        Err(e) => {
            eprintln!("extract: {e}");
            1
        }
    }
}
