//! Restore full FSB5 parser when the stub is detected.
//!
//! Priority:
//! 1. recovery/fsb.z0+z1+z2 (zlib+base64 offline chunks)
//! 2. recovery/fsb.rs plaintext
//! 3. Download from known-good commit (network)

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
    let src = Path::new(&manifest).join("src/fsb.rs");

    let current = fs::read_to_string(&src).unwrap_or_default();
    let needs_restore = current.contains("NotRestored")
        || current.contains("PLACEHOLDER")
        || current.len() < 10_000;

    if !needs_restore {
        println!("cargo:rerun-if-changed=src/fsb.rs");
        println!("cargo:rerun-if-changed=recovery");
        return;
    }

    if let Some(body) = recover(&manifest) {
        if let Err(e) = fs::write(&src, &body) {
            eprintln!("cargo:warning=could not write src/fsb.rs: {e}");
        } else {
            println!("cargo:warning=restored src/fsb.rs ({} bytes)", body.len());
        }
    } else {
        eprintln!("cargo:warning=FSB5 full parser not restored — export FSB may fail verify");
    }

    println!("cargo:rerun-if-changed=src/fsb.rs");
    println!("cargo:rerun-if-changed=recovery");
    println!("cargo:rerun-if-changed=build.rs");
}

fn recover(manifest: &str) -> Option<String> {
    if let Some(s) = from_zlib_chunks(manifest) {
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

fn from_zlib_chunks(manifest: &str) -> Option<String> {
    let dir = Path::new(manifest).join("recovery");
    let mut b64 = String::new();
    for i in 0..8 {
        let p = dir.join(format!("fsb.z{i}"));
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
