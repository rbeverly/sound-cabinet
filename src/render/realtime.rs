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
    play_realtime_inner(engine, false)
}

/// Play with optional VU meter display.
pub fn play_realtime_vu(engine: Engine) -> Result<()> {
    play_realtime_inner(engine, true)
}

fn play_realtime_inner(engine: Engine, show_vu: bool) -> Result<()> {
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

    if show_vu {
        eprintln!(); // blank line separator after "Playing..."
    }

    // Track number of VU lines printed for terminal cleanup
    let mut vu_lines: usize = 0;

    // Wait until the engine finishes
    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let eng = engine.lock().unwrap();

        if show_vu {
            let vu_meters = eng.voice_vu();
            if !vu_meters.is_empty() {
                // Clear previous VU meter lines by moving cursor up
                for _ in 0..vu_lines {
                    eprint!("\x1b[A\r\x1b[2K");
                }

                let mut entries: Vec<_> = vu_meters.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));

                vu_lines = entries.len();
                for (name, vu) in &entries {
                    let level_db = vu.level_db();
                    let peak_db = vu.peak_hold_db();
                    let bar_width = ((level_db + 60.0) / 60.0 * 40.0).clamp(0.0, 40.0) as usize;
                    let peak_pos = ((peak_db + 60.0) / 60.0 * 40.0).clamp(0.0, 39.0) as usize;

                    // Build the bar with color gradient and peak hold indicator
                    let mut bar = String::new();
                    for i in 0..40 {
                        if i == peak_pos && peak_db > -60.0 && i >= bar_width {
                            // Peak hold marker (beyond current level)
                            let color = if i >= 36 {
                                "\x1b[91m" // red
                            } else if i >= 30 {
                                "\x1b[93m" // yellow
                            } else {
                                "\x1b[97m" // white
                            };
                            bar.push_str(color);
                            bar.push('\u{2502}'); // thin vertical bar as peak marker
                        } else if i < bar_width {
                            let color = if i >= 36 {
                                "\x1b[91m" // bright red: > -6 dB
                            } else if i >= 30 {
                                "\x1b[93m" // bright yellow: -6 to -15 dB
                            } else if i >= 20 {
                                "\x1b[92m" // bright green: -15 to -30 dB
                            } else {
                                "\x1b[32m" // green: < -30 dB
                            };
                            bar.push_str(color);
                            bar.push('\u{2588}');
                        } else {
                            bar.push_str("\x1b[90m"); // dark gray
                            bar.push('\u{2591}');
                        }
                    }
                    bar.push_str("\x1b[0m"); // reset

                    // Status indicator
                    let status = if peak_db > -1.0 {
                        " \x1b[91;1mCLIP\x1b[0m"
                    } else if level_db < -60.0 && peak_db < -60.0 {
                        ""
                    } else {
                        ""
                    };

                    // Name color: dim if silent, white if active
                    let name_color = if level_db < -50.0 { "\x1b[90m" } else { "\x1b[97m" };

                    eprint!(
                        "  {}{:<14}\x1b[0m {} {:>6.1} dB{}\r\n",
                        name_color, name, bar, level_db, status
                    );
                }
                use std::io::Write;
                let _ = std::io::stderr().flush();
            }
        }

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
