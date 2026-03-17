use std::path::Path;

use anyhow::Result;
use hound::{SampleFormat, WavSpec, WavWriter};

use crate::engine::Engine;

/// Render the engine's scheduled events to a WAV file.
pub fn render_to_wav(engine: &mut Engine, path: &Path) -> Result<()> {
    let sample_rate = engine.sample_rate as u32;
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;
    let mut buffer = vec![0.0f32; 1024];

    while !engine.is_finished() {
        engine.render_samples(&mut buffer);
        for &sample in &buffer {
            // Clamp and convert to i16
            let clamped = sample.clamp(-1.0, 1.0);
            let int_sample = (clamped * i16::MAX as f32) as i16;
            writer.write_sample(int_sample)?;
        }
    }

    writer.finalize()?;
    Ok(())
}
