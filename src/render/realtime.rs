use std::sync::{Arc, Mutex};

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleFormat, StreamConfig};

use crate::engine::Engine;

/// Preferred buffer size in frames. Larger = more latency but fewer underruns.
/// 2048 frames at 44100 Hz ≈ 46ms latency — fine for non-interactive playback.
const PREFERRED_BUFFER_FRAMES: u32 = 2048;

/// Play the engine's scheduled events through the default audio output.
pub fn play_realtime(engine: Engine) -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No output audio device found"))?;

    let supported = device.default_output_config()?;
    let sample_format = supported.sample_format();
    let channels = supported.channels();

    // Use a fixed buffer size to avoid underruns with complex compositions
    let stream_config = StreamConfig {
        channels,
        sample_rate: supported.sample_rate(),
        buffer_size: BufferSize::Fixed(PREFERRED_BUFFER_FRAMES),
    };

    let engine = Arc::new(Mutex::new(engine));
    let engine_clone = Arc::clone(&engine);

    // Pre-allocate the mono render buffer outside the callback.
    // NEVER allocate in the audio callback — heap allocation can block.
    let max_mono_frames = PREFERRED_BUFFER_FRAMES as usize;
    let mono_buf = Arc::new(Mutex::new(vec![0.0f32; max_mono_frames]));
    let mono_buf_clone = Arc::clone(&mono_buf);

    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| {
                let mut eng = engine_clone.lock().unwrap();
                let mut buf = mono_buf_clone.lock().unwrap();
                let frame_count = data.len() / channels as usize;

                // Ensure our pre-allocated buffer is large enough
                // (resize is a no-op if already big enough)
                if buf.len() < frame_count {
                    buf.resize(frame_count, 0.0);
                }

                eng.render_samples(&mut buf[..frame_count]);

                // Interleave mono to all output channels, clamping to prevent driver clipping
                for (i, frame) in data.chunks_mut(channels as usize).enumerate() {
                    let sample = if i < frame_count { buf[i].clamp(-1.0, 1.0) } else { 0.0 };
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

    let supported = device.default_output_config()?;
    let sample_format = supported.sample_format();
    let channels = supported.channels();

    let stream_config = StreamConfig {
        channels,
        sample_rate: supported.sample_rate(),
        buffer_size: BufferSize::Fixed(PREFERRED_BUFFER_FRAMES),
    };

    let engine_clone = Arc::clone(&engine);

    let max_mono_frames = PREFERRED_BUFFER_FRAMES as usize;
    let mono_buf = Arc::new(Mutex::new(vec![0.0f32; max_mono_frames]));
    let mono_buf_clone = Arc::clone(&mono_buf);

    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| {
                let mut eng = engine_clone.lock().unwrap();
                let mut buf = mono_buf_clone.lock().unwrap();
                let frame_count = data.len() / channels as usize;

                if buf.len() < frame_count {
                    buf.resize(frame_count, 0.0);
                }

                eng.render_samples(&mut buf[..frame_count]);

                for (i, frame) in data.chunks_mut(channels as usize).enumerate() {
                    let sample = if i < frame_count { buf[i].clamp(-1.0, 1.0) } else { 0.0 };
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
