//! Restore full FSB5 parser and S7.22 truncated runtime/editor sources.
//!
//! Priority for each target:
//! 1. recovery/*.part* concatenated offline parts
//! 2. recovery/*.z* offline zlib+base64 chunks
//! 3. recovery/*.rs plaintext
//! 4. (FSB only) download from known-good commit

use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;

const KNOWN_GOOD_URL: &str = concat!(
    "https://raw.githubusercontent.com/SimonTek27/RaMOD/",
    "d0e38a10e0ad438bbea79095292b3bbd0218c953/src/fsb.rs",
);

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");

    restore_fsb(&manifest);
    restore_from_chunks(
        &manifest,
        "runtime/src/runtime_core_b.rs",
        "core_b",
        |s| s.contains("play_event") && s.contains("set_bus_send_level") && s.len() > 12_000,
        12_000,
    );
    restore_from_chunks(
        &manifest,
        "editor/src/audio_bridge.rs",
        "ab",
        |s| s.contains("sync_project") && s.contains("set_bus_send_reverb") && s.len() > 15_000,
        15_000,
    );

    println!("cargo:rerun-if-changed=src/fsb.rs");
    println!("cargo:rerun-if-changed=runtime/src/runtime_core_b.rs");
    println!("cargo:rerun-if-changed=editor/src/audio_bridge.rs");
    println!("cargo:rerun-if-changed=recovery");
    println!("cargo:rerun-if-changed=build.rs");
}

fn restore_fsb(manifest: &str) {
    let src = Path::new(manifest).join("src/fsb.rs");
    let current = fs::read_to_string(&src).unwrap_or_default();
    let needs = current.contains("NotRestored")
        || current.contains("PLACEHOLDER")
        || current.len() < 10_000;
    if !needs {
        return;
    }
    if let Some(body) = recover_fsb(manifest) {
        if let Err(e) = fs::write(&src, &body) {
            eprintln!("cargo:warning=could not write src/fsb.rs: {e}");
        } else {
            println!("cargo:warning=restored src/fsb.rs ({} bytes)", body.len());
        }
    } else {
        eprintln!("cargo:warning=FSB5 full parser not restored — export FSB may fail verify");
    }
}

fn restore_from_chunks(
    manifest: &str,
    rel_path: &str,
    chunk_prefix: &str,
    is_good: fn(&str) -> bool,
    min_len: usize,
) {
    let path = Path::new(manifest).join(rel_path);
    let current = fs::read_to_string(&path).unwrap_or_default();
    let needs = current.contains("PLACEHOLDER")
        || current.contains("remaining AudioBridge methods")
        || current.len() < min_len
        || !is_good(&current);
    if !needs {
        return;
    }
    if let Some(body) = from_parts(manifest, chunk_prefix).or_else(|| from_zlib_prefix(manifest, chunk_prefix))
    {
        if is_good(&body) {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            match fs::write(&path, &body) {
                Ok(()) => println!(
                    "cargo:warning=restored {} ({} bytes from recovery/{}.part*/z*)",
                    rel_path,
                    body.len(),
                    chunk_prefix
                ),
                Err(e) => eprintln!("cargo:warning=could not write {rel_path}: {e}"),
            }
            return;
        }
    }
    let plain = Path::new(manifest).join("recovery").join(format!("{chunk_prefix}.rs"));
    if let Ok(s) = fs::read_to_string(&plain) {
        if is_good(&s) {
            let _ = fs::write(&path, &s);
            println!("cargo:warning=restored {rel_path} from recovery/{chunk_prefix}.rs");
        }
    }
}

fn recover_fsb(manifest: &str) -> Option<String> {
    if let Some(s) = from_zlib_prefix(manifest, "fsb") {
        if is_full_parser(&s) {
            println!("cargo:warning=using recovery/fsb.z* offline blob");
            return Some(s);
        }
    }
    let plain = Path::new(manifest).join("recovery/fsb.rs");
    if let Ok(s) = fs::read_to_string(&plain) {
        if is_full_parser(&s) {
            println!("cargo:warning=using recovery/fsb.rs");
            return Some(s);
        }
    }
    if let Some(s) = download(KNOWN_GOOD_URL) {
        if is_full_parser(&s) {
            let cache = Path::new(manifest).join("recovery");
            let _ = fs::create_dir_all(&cache);
            let _ = fs::write(cache.join("fsb.rs"), &s);
            return Some(s);
        }
    }
    None
}

fn from_parts(manifest: &str, prefix: &str) -> Option<String> {
    let dir = Path::new(manifest).join("recovery");
    let mut out = String::new();
    for i in 0..16 {
        let p = dir.join(format!("{prefix}.part{i}"));
        if !p.exists() {
            if i == 0 {
                return None;
            }
            break;
        }
        out.push_str(&fs::read_to_string(&p).ok()?);
    }
    Some(out)
}

fn from_zlib_prefix(manifest: &str, prefix: &str) -> Option<String> {
    let dir = Path::new(manifest).join("recovery");
    let mut b64 = String::new();
    for i in 0..8 {
        let p = dir.join(format!("{prefix}.z{i}"));
        if !p.exists() {
            if i == 0 {
                return None;
            }
            break;
        }
        b64.push_str(&fs::read_to_string(&p).ok()?.trim());
    }
    let compressed = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        b64.trim(),
    )
    .ok()?;
    let mut dec = flate2::read::ZlibDecoder::new(compressed.as_slice());
    let mut out = String::new();
    dec.read_to_string(&mut out).ok()?;
    Some(out)
}

fn is_full_parser(s: &str) -> bool {
    s.contains("pub struct Fsb5")
        && s.contains("fn sample_to_wav")
        && s.contains("fn parse")
        && !s.contains("NotRestored")
        && s.len() > 10_000
}

fn download(url: &str) -> Option<String> {
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "30", url])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}
