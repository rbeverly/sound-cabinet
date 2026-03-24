use std::path::Path;

use anyhow::Result;
use hound::{SampleFormat, WavSpec, WavWriter};

use crate::engine::effects::BrickwallLimiter;
use crate::engine::Engine;
use crate::render::lufs::{measure_lufs, true_peak_dbfs};

/// Render the engine's scheduled events to a WAV file.
/// If `target_lufs` is Some, normalizes to that loudness target.
pub fn render_to_wav(engine: &mut Engine, path: &Path, target_lufs: Option<f64>) -> Result<()> {
    let sample_rate = engine.sample_rate as u32;

    // First pass: render all samples to memory
    let mut all_samples: Vec<f32> = Vec::new();
    let mut buffer = vec![0.0f32; 1024];

    while !engine.is_finished() {
        engine.render_samples(&mut buffer);
        all_samples.extend_from_slice(&buffer);
    }

    // Flush the master bus limiter lookahead tail
    let tail = engine.flush_master();
    all_samples.extend_from_slice(&tail);

    // Trim trailing silence (limiter lookahead can add a few ms of near-silence)
    while all_samples.last().map_or(false, |&s| s.abs() < 1e-8) {
        all_samples.pop();
    }

    // Measure loudness and true peak
    let lufs = measure_lufs(&all_samples, sample_rate as f64);
    let peak = true_peak_dbfs(&all_samples);

    eprintln!("  Integrated loudness: {:.1} LUFS", lufs);
    eprintln!("  True peak: {:.1} dBFS", peak);

    // Optional LUFS normalization
    if let Some(target) = target_lufs {
        if lufs.is_finite() {
            let gain_db = target - lufs;
            let gain_linear = 10.0_f64.powf(gain_db / 20.0) as f32;

            // Apply gain
            for sample in all_samples.iter_mut() {
                *sample *= gain_linear;
            }

            // Always re-limit after normalization to prevent clipping.
            // Use -1.0 dBFS ceiling to leave headroom for inter-sample peaks
            // (true peak can exceed sample peak by up to ~0.5 dB).
            // TODO: upgrade to a true peak limiter with 4x oversampled detection
            // so we can use a tighter ceiling without inter-sample clipping.
            let ceiling = 10.0_f32.powf(-1.0 / 20.0);
            let mut limiter = BrickwallLimiter::new(ceiling, 0.1, sample_rate as f64);
            for chunk in all_samples.chunks_mut(1024) {
                limiter.process(chunk);
            }
            let mut tail = Vec::new();
            limiter.flush(&mut tail);
            all_samples.extend_from_slice(&tail);

            // Re-measure after normalization + limiting
            let new_lufs = measure_lufs(&all_samples, sample_rate as f64);
            let final_peak = true_peak_dbfs(&all_samples);
            eprintln!("  Normalized to {:.1} LUFS (gain: {:+.1} dB)", new_lufs, gain_db);
            eprintln!("  Final true peak: {:.1} dBFS", final_peak);
        } else {
            eprintln!("  Cannot normalize: signal is silent");
        }
    }

    // Write to WAV
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;
    for &sample in &all_samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let int_sample = (clamped * i16::MAX as f32) as i16;
        writer.write_sample(int_sample)?;
    }
    writer.finalize()?;

    Ok(())
}
