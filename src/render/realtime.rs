use std::sync::{Arc, Mutex};

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;

use crate::engine::Engine;

/// Play the engine's scheduled events through the default audio output.
pub fn play_realtime(engine: Engine) -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No output audio device found"))?;

    let config = device.default_output_config()?;
    let sample_format = config.sample_format();
    let stream_config = config.into();

    let engine = Arc::new(Mutex::new(engine));
    let engine_clone = Arc::clone(&engine);

    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| {
                let mut eng = engine_clone.lock().unwrap();
                // Render mono and interleave to stereo
                let frame_count = data.len() / 2;
                let mut mono_buf = vec![0.0f32; frame_count];
                eng.render_samples(&mut mono_buf);

                // Interleave mono to stereo (or however many channels)
                for (i, frame) in data.chunks_mut(2).enumerate() {
                    let sample = if i < mono_buf.len() { mono_buf[i] } else { 0.0 };
                    for ch in frame.iter_mut() {
                        *ch = sample;
                    }
                }
            },
            |err| eprintln!("Audio stream error: {err}"),
            None,
        )?,
        _ => return Err(anyhow::anyhow!("Unsupported sample format: {sample_format:?}")),
    };

    stream.play()?;

    // Wait until the engine finishes
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let eng = engine.lock().unwrap();
        if eng.is_finished() {
            break;
        }
    }

    Ok(())
}

/// Play with a channel-fed engine for streaming mode.
pub fn play_streaming(
    engine: Arc<Mutex<Engine>>,
    shutdown_rx: crossbeam_channel::Receiver<()>,
) -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No output audio device found"))?;

    let config = device.default_output_config()?;
    let sample_format = config.sample_format();
    let stream_config = config.into();

    let engine_clone = Arc::clone(&engine);

    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| {
                let mut eng = engine_clone.lock().unwrap();
                let frame_count = data.len() / 2;
                let mut mono_buf = vec![0.0f32; frame_count];
                eng.render_samples(&mut mono_buf);

                for (i, frame) in data.chunks_mut(2).enumerate() {
                    let sample = if i < mono_buf.len() { mono_buf[i] } else { 0.0 };
                    for ch in frame.iter_mut() {
                        *ch = sample;
                    }
                }
            },
            |err| eprintln!("Audio stream error: {err}"),
            None,
        )?,
        _ => return Err(anyhow::anyhow!("Unsupported sample format: {sample_format:?}")),
    };

    stream.play()?;

    // Wait for shutdown signal
    let _ = shutdown_rx.recv();

    Ok(())
}
