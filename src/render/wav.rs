use std::path::Path;

use anyhow::Result;
use hound::{SampleFormat, WavSpec, WavWriter};

use crate::engine::effects::BrickwallLimiter;
use crate::engine::Engine;
use crate::render::lufs::{measure_lufs, true_peak_dbfs};

/// Render the engine's scheduled events to a stereo WAV file.
/// If `target_lufs` is Some, normalizes to that loudness target.
pub fn render_to_wav(engine: &mut Engine, path: &Path, target_lufs: Option<f64>) -> Result<()> {
    let sample_rate = engine.sample_rate as u32;

    // First pass: render all samples to memory (stereo)
    let mut all_left: Vec<f32> = Vec::new();
    let mut all_right: Vec<f32> = Vec::new();
    let mut left_buf = vec![0.0f32; 1024];
    let mut right_buf = vec![0.0f32; 1024];

    while !engine.is_finished() {
        engine.render_samples(&mut left_buf, &mut right_buf);
        all_left.extend_from_slice(&left_buf);
        all_right.extend_from_slice(&right_buf);
    }

    // Flush the master bus limiter lookahead tail
    let (left_tail, right_tail) = engine.flush_master();
    all_left.extend_from_slice(&left_tail);
    all_right.extend_from_slice(&right_tail);

    // Trim trailing silence (limiter lookahead can add a few ms of near-silence)
    while all_left.last().map_or(false, |&s| s.abs() < 1e-8)
        && all_right.last().map_or(false, |&s| s.abs() < 1e-8)
    {
        all_left.pop();
        all_right.pop();
    }

    // Measure loudness and true peak
    let lufs = measure_lufs(&all_left, &all_right, sample_rate as f64);
    let peak = true_peak_dbfs(&all_left, &all_right);

    eprintln!("  Integrated loudness: {:.1} LUFS", lufs);
    eprintln!("  True peak: {:.1} dBFS", peak);

    // Optional LUFS normalization
    if let Some(target) = target_lufs {
        if target > 0.0 {
            eprintln!("  Warning: --lufs {} is positive — did you mean --lufs {}? LUFS targets are always negative (e.g., -14 for Spotify).", target, -target.abs());
        }
        if lufs.is_finite() {
            let gain_db = target - lufs;
            let gain_linear = 10.0_f64.powf(gain_db / 20.0) as f32;

            // Apply gain to both channels
            for sample in all_left.iter_mut() {
                *sample *= gain_linear;
            }
            for sample in all_right.iter_mut() {
                *sample *= gain_linear;
            }

            // Always re-limit after normalization to prevent clipping.
            // Linked stereo: compute needed reduction from max(|L|, |R|),
            // apply same reduction to both channels.
            let ceiling = 10.0_f32.powf(-1.0 / 20.0);
            let mut limiter_l = BrickwallLimiter::new(ceiling, 0.1, sample_rate as f64);
            let mut limiter_r = BrickwallLimiter::new(ceiling, 0.1, sample_rate as f64);
            for (l_chunk, r_chunk) in all_left.chunks_mut(1024).zip(all_right.chunks_mut(1024)) {
                limiter_l.process(l_chunk);
                limiter_r.process(r_chunk);
            }
            let mut left_tail = Vec::new();
            let mut right_tail = Vec::new();
            limiter_l.flush(&mut left_tail);
            limiter_r.flush(&mut right_tail);
            all_left.extend_from_slice(&left_tail);
            all_right.extend_from_slice(&right_tail);

            // Re-measure after normalization + limiting
            let new_lufs = measure_lufs(&all_left, &all_right, sample_rate as f64);
            let final_peak = true_peak_dbfs(&all_left, &all_right);
            eprintln!("  Normalized to {:.1} LUFS (gain: {:+.1} dB)", new_lufs, gain_db);
            eprintln!("  Final true peak: {:.1} dBFS", final_peak);
        } else {
            eprintln!("  Cannot normalize: signal is silent");
        }
    }

    // Write to stereo WAV
    let spec = WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let len = all_left.len().min(all_right.len());
    let mut writer = WavWriter::create(path, spec)?;
    for i in 0..len {
        let l_clamped = all_left[i].clamp(-1.0, 1.0);
        let r_clamped = all_right[i].clamp(-1.0, 1.0);
        writer.write_sample((l_clamped * i16::MAX as f32) as i16)?;
        writer.write_sample((r_clamped * i16::MAX as f32) as i16)?;
    }
    writer.finalize()?;

    Ok(())
}
