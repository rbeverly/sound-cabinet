use std::sync::{Arc, Mutex};

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleFormat, StreamConfig};

use crate::engine::Engine;

/// Preferred buffer size in frames. Larger = more latency but fewer underruns.
/// 2048 frames at 44100 Hz ≈ 46ms latency — fine for non-interactive playback.
const PREFERRED_BUFFER_FRAMES: u32 = 2048;

/// Monitoring mode flags.
#[derive(Clone, Copy, Default)]
pub struct MonitorFlags {
    pub show_vu: bool,
    pub subfold: bool,
    pub env_noise: Option<EnvNoiseProfile>,
}

/// Environmental noise profile for --env monitoring.
#[derive(Clone, Copy)]
pub enum EnvNoiseProfile {
    Car,
    Cafe,
    Subway,
}

/// Play the engine's scheduled events through the default audio output.
pub fn play_realtime(engine: Engine) -> Result<()> {
    play_realtime_inner(engine, MonitorFlags::default())
}

/// Play with optional VU meter display.
pub fn play_realtime_vu(engine: Engine) -> Result<()> {
    play_realtime_inner(engine, MonitorFlags { show_vu: true, ..Default::default() })
}

/// Play with monitoring flags.
pub fn play_realtime_monitored(engine: Engine, flags: MonitorFlags) -> Result<()> {
    play_realtime_inner(engine, flags)
}

/// Sub-bass fold-up state for monitoring.
struct SubFoldState {
    lp_l: f64,  // one-pole lowpass state for left
    lp_r: f64,  // one-pole lowpass state for right
    alpha: f64, // filter coefficient for ~80 Hz cutoff
}

impl SubFoldState {
    fn new(sample_rate: f64) -> Self {
        let alpha = (2.0 * std::f64::consts::PI * 80.0) / (2.0 * std::f64::consts::PI * 80.0 + sample_rate);
        SubFoldState { lp_l: 0.0, lp_r: 0.0, alpha }
    }

    /// Extract sub-bass, rectify (shift up 1 octave), and mix back at low level.
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let mix = 0.3; // blend level for the folded-up sub-bass
        for i in 0..left.len() {
            // Extract sub-bass via one-pole lowpass
            self.lp_l += self.alpha * (left[i] as f64 - self.lp_l);
            self.lp_r += self.alpha * (right[i] as f64 - self.lp_r);

            // Full-wave rectify = shift up 1 octave, do it twice for 2 octaves
            let fold_l = self.lp_l.abs().abs() as f32; // |sub| = +1 octave
            let fold_r = self.lp_r.abs().abs() as f32;

            // Mix the folded sub-bass back into the output
            left[i] += fold_l * mix;
            right[i] += fold_r * mix;
        }
    }
}

fn play_realtime_inner(engine: Engine, flags: MonitorFlags) -> Result<()> {
    let show_vu = flags.show_vu;
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No output audio device found"))?;

    let supported = device.default_output_config()?;
    let sample_format = supported.sample_format();
    let channels = supported.channels();
    let sample_rate = supported.sample_rate().0 as f64;

    // Use a fixed buffer size to avoid underruns with complex compositions
    let stream_config = StreamConfig {
        channels,
        sample_rate: supported.sample_rate(),
        buffer_size: BufferSize::Fixed(PREFERRED_BUFFER_FRAMES),
    };

    let engine = Arc::new(Mutex::new(engine));
    let engine_clone = Arc::clone(&engine);

    // Pre-allocate stereo render buffers outside the callback.
    // NEVER allocate in the audio callback — heap allocation can block.
    let max_frames = PREFERRED_BUFFER_FRAMES as usize;
    let left_buf = Arc::new(Mutex::new(vec![0.0f32; max_frames]));
    let right_buf = Arc::new(Mutex::new(vec![0.0f32; max_frames]));
    let left_clone = Arc::clone(&left_buf);
    let right_clone = Arc::clone(&right_buf);

    // Monitoring state
    let subfold_state = if flags.subfold {
        Some(Arc::new(Mutex::new(SubFoldState::new(sample_rate))))
    } else {
        None
    };
    let subfold_clone = subfold_state.clone();

    let env_noise = flags.env_noise;

    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| {
                let mut eng = engine_clone.lock().unwrap();
                let mut lbuf = left_clone.lock().unwrap();
                let mut rbuf = right_clone.lock().unwrap();
                let frame_count = data.len() / channels as usize;

                // Ensure our pre-allocated buffers are large enough
                if lbuf.len() < frame_count {
                    lbuf.resize(frame_count, 0.0);
                }
                if rbuf.len() < frame_count {
                    rbuf.resize(frame_count, 0.0);
                }

                eng.render_samples(&mut lbuf[..frame_count], &mut rbuf[..frame_count]);

                // Sub-bass fold-up monitoring: pitch-shift sub-bass up for headphone monitoring
                if let Some(ref sf) = subfold_clone {
                    if let Ok(mut state) = sf.lock() {
                        state.process(&mut lbuf[..frame_count], &mut rbuf[..frame_count]);
                    }
                }

                // Interleave stereo to output channels, clamping to prevent driver clipping
                // Also add environmental noise if enabled
                let noise_level = match env_noise {
                    Some(EnvNoiseProfile::Car) => 0.08,    // heavy low-mid rumble
                    Some(EnvNoiseProfile::Cafe) => 0.04,   // lighter broadband
                    Some(EnvNoiseProfile::Subway) => 0.12, // very heavy
                    None => 0.0,
                };
                for (i, frame) in data.chunks_mut(channels as usize).enumerate() {
                    let mut l = if i < frame_count { lbuf[i].clamp(-1.0, 1.0) } else { 0.0 };
                    let mut r = if i < frame_count { rbuf[i].clamp(-1.0, 1.0) } else { 0.0 };

                    // Mix in environmental noise (monitoring only)
                    if noise_level > 0.0 {
                        // Simple white noise from hash function (no allocations)
                        let seed = (i as u64).wrapping_mul(2654435761).wrapping_add(frame_count as u64);
                        let noise = ((seed & 0xFFFF) as f32 / 32768.0) - 1.0;
                        // Brown-ish filtering: just average with previous for low-frequency character
                        l += noise * noise_level;
                        r += noise * noise_level * 0.95; // slight L/R decorrelation
                    }
                    match frame.len() {
                        1 => {
                            // Mono output device: downmix
                            frame[0] = (l + r) * 0.5;
                        }
                        _ => {
                            // Stereo or more: L, R, then silence for extra channels
                            frame[0] = l;
                            if frame.len() > 1 { frame[1] = r; }
                            for ch in frame.iter_mut().skip(2) {
                                *ch = 0.0;
                            }
                        }
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

    let max_frames = PREFERRED_BUFFER_FRAMES as usize;
    let left_buf = Arc::new(Mutex::new(vec![0.0f32; max_frames]));
    let right_buf = Arc::new(Mutex::new(vec![0.0f32; max_frames]));
    let left_clone = Arc::clone(&left_buf);
    let right_clone = Arc::clone(&right_buf);

    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| {
                let mut eng = engine_clone.lock().unwrap();
                let mut lbuf = left_clone.lock().unwrap();
                let mut rbuf = right_clone.lock().unwrap();
                let frame_count = data.len() / channels as usize;

                if lbuf.len() < frame_count {
                    lbuf.resize(frame_count, 0.0);
                }
                if rbuf.len() < frame_count {
                    rbuf.resize(frame_count, 0.0);
                }

                eng.render_samples(&mut lbuf[..frame_count], &mut rbuf[..frame_count]);

                for (i, frame) in data.chunks_mut(channels as usize).enumerate() {
                    let l = if i < frame_count { lbuf[i].clamp(-1.0, 1.0) } else { 0.0 };
                    let r = if i < frame_count { rbuf[i].clamp(-1.0, 1.0) } else { 0.0 };
                    match frame.len() {
                        1 => {
                            frame[0] = (l + r) * 0.5;
                        }
                        _ => {
                            frame[0] = l;
                            if frame.len() > 1 { frame[1] = r; }
                            for ch in frame.iter_mut().skip(2) {
                                *ch = 0.0;
                            }
                        }
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
