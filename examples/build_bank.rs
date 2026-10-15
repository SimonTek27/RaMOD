//! Build an FSB5 bank from a generated sine wave, then read it back.
//!
//! Run with: `cargo run --example build_bank`

use ramod::{Fsb5, Fsb5Builder, PcmSample, SoundFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate a quarter second of a 440 Hz sine wave as 16-bit mono PCM.
    let rate = 44_100u32;
    let frames = rate / 4;
    let mut pcm = Vec::with_capacity(frames as usize * 2);
    for i in 0..frames {
        let t = i as f32 / rate as f32;
        let value = (0.5 * 32_767.0 * (std::f32::consts::TAU * 440.0 * t).sin()) as i16;
        pcm.extend_from_slice(&value.to_le_bytes());
    }

    // Build a single-sample bank and serialise it.
    let sample = PcmSample::new("tone", rate, 1, pcm);
    let bank = Fsb5Builder::new(SoundFormat::Pcm16)
        .with_sample(sample)
        .build()?;
    std::fs::write("tone.fsb", &bank)?;

    // The writer self-verifies, but we can parse the bytes again ourselves.
    let parsed = Fsb5::parse(&bank)?;
    let read = &parsed.samples[0];
    println!(
        "tone.fsb: {} sample(s), '{}' {} Hz, {} ch, {} frames, {:?}",
        parsed.len(),
        read.name,
        read.frequency,
        read.channels,
        read.num_frames,
        parsed.format
    );

    Ok(())
}
