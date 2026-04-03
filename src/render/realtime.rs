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

/// A single filtered noise layer with its own LCG state and filter.
struct NoiseLayer {
    rng_state: u64,
    lp_l: f64,
    lp_r: f64,
    alpha: f64,    // lowpass coefficient
    hp_l: f64,     // highpass state
    hp_r: f64,
    hp_alpha: f64, // highpass coefficient (0 = no highpass)
    level: f32,
}

impl NoiseLayer {
    fn new(seed: u64, lp_freq: f64, hp_freq: f64, level: f32, sample_rate: f64) -> Self {
        let lp_alpha = (2.0 * std::f64::consts::PI * lp_freq) / (2.0 * std::f64::consts::PI * lp_freq + sample_rate);
        let hp_alpha = if hp_freq > 0.0 {
            1.0 - (2.0 * std::f64::consts::PI * hp_freq) / (2.0 * std::f64::consts::PI * hp_freq + sample_rate)
        } else {
            0.0 // no highpass
        };
        NoiseLayer {
            rng_state: seed,
            lp_l: 0.0, lp_r: 0.0,
            alpha: lp_alpha,
            hp_l: 0.0, hp_r: 0.0,
            hp_alpha,
            level,
        }
    }

    #[inline]
    fn next_white(&mut self) -> (f64, f64) {
        self.rng_state = self.rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let l = ((self.rng_state >> 33) as i32 as f64) / (i32::MAX as f64);
        self.rng_state = self.rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r = ((self.rng_state >> 33) as i32 as f64) / (i32::MAX as f64);
        (l, r)
    }

    #[inline]
    fn next(&mut self) -> (f32, f32) {
        let (wl, wr) = self.next_white();

        // Lowpass
        self.lp_l += self.alpha * (wl - self.lp_l);
        self.lp_r += self.alpha * (wr - self.lp_r);

        let mut l = self.lp_l;
        let mut r = self.lp_r;

        // Optional highpass (bandpass = lowpass then highpass)
        if self.hp_alpha > 0.0 {
            let prev_hp_l = self.hp_l;
            let prev_hp_r = self.hp_r;
            self.hp_l = self.hp_alpha * (prev_hp_l + l - self.lp_l);
            self.hp_r = self.hp_alpha * (prev_hp_r + r - self.lp_r);
            // Simple DC-blocking highpass approximation
            l = l - self.hp_l;
            r = r - self.hp_r;
        }

        (l as f32 * self.level, r as f32 * self.level)
    }
}

/// Stateful environmental noise generator for monitoring.
/// Uses multiple filtered noise layers to approximate real-world noise spectra.
struct EnvNoiseGen {
    layers: Vec<NoiseLayer>,
}

impl EnvNoiseGen {
    fn new(profile: EnvNoiseProfile, sample_rate: f64) -> Self {
        let layers = match profile {
            EnvNoiseProfile::Car => vec![
                // Engine rumble: deep, 40-150 Hz
                NoiseLayer::new(0xDEADBEEF11111111, 150.0, 40.0, 0.25, sample_rate),
                // Tire noise: peaks around 800-1200 Hz
                NoiseLayer::new(0xCAFEBABE22222222, 1200.0, 300.0, 0.20, sample_rate),
                // Wind noise: broadband above 500 Hz
                NoiseLayer::new(0x1234567833333333, 6000.0, 500.0, 0.12, sample_rate),
                // A/C hiss: high frequency
                NoiseLayer::new(0xABCDEF0044444444, 8000.0, 2000.0, 0.06, sample_rate),
            ],
            EnvNoiseProfile::Cafe => vec![
                // Room tone: low ambient
                NoiseLayer::new(0x1111111111111111, 400.0, 60.0, 0.08, sample_rate),
                // Speech frequencies: mid-heavy chatter
                NoiseLayer::new(0x2222222222222222, 3000.0, 300.0, 0.15, sample_rate),
                // Coffee machine / clinking: high mid
                NoiseLayer::new(0x3333333333333333, 6000.0, 1000.0, 0.06, sample_rate),
            ],
            EnvNoiseProfile::Subway => vec![
                // Tunnel resonance: very deep
                NoiseLayer::new(0xAAAAAAAAAAAAAAAA, 200.0, 30.0, 0.30, sample_rate),
                // Rail rumble: low-mid
                NoiseLayer::new(0xBBBBBBBBBBBBBBBB, 800.0, 100.0, 0.20, sample_rate),
                // Brake screech / broadband: mid-high
                NoiseLayer::new(0xCCCCCCCCCCCCCCCC, 4000.0, 500.0, 0.15, sample_rate),
                // Electrical hum
                NoiseLayer::new(0xDDDDDDDDDDDDDDDD, 200.0, 50.0, 0.10, sample_rate),
            ],
        };
        EnvNoiseGen { layers }
    }

    /// Generate next noise sample pair (L, R).
    #[inline]
    fn next(&mut self) -> (f32, f32) {
        let mut l = 0.0f32;
        let mut r = 0.0f32;
        for layer in &mut self.layers {
            let (nl, nr) = layer.next();
            l += nl;
            r += nr;
        }
        (l, r)
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

    let env_noise_gen = flags.env_noise.map(|profile| {
        Arc::new(Mutex::new(EnvNoiseGen::new(profile, sample_rate)))
    });
    let env_noise_clone = env_noise_gen.clone();

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
                // Lock env noise generator if present (once per buffer, not per sample)
                let mut env_gen_lock = env_noise_clone.as_ref().map(|g| g.lock().unwrap());

                for (i, frame) in data.chunks_mut(channels as usize).enumerate() {
                    let mut l = if i < frame_count { lbuf[i].clamp(-1.0, 1.0) } else { 0.0 };
                    let mut r = if i < frame_count { rbuf[i].clamp(-1.0, 1.0) } else { 0.0 };

                    // Mix in environmental noise (monitoring only)
                    if let Some(ref mut gen) = env_gen_lock {
                        let (nl, nr) = gen.next();
                        l += nl;
                        r += nr;
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
