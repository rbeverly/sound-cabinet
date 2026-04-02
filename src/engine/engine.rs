use std::collections::HashMap;

use anyhow::Result;
use fundsp::hacker::*;

use crate::dsl::ast::{Command, DefKind, Expr};
use crate::dsl::parser::{extract_voice_label, resolve_chord};
use crate::engine::effects::MasterBus;
use crate::engine::graph::{build_graph, extract_arp, extract_bus, extract_sidechain, extract_swell, strip_bus, strip_sidechain, strip_swell, substitute_freq, substitute_var, SidechainConfig};

/// Built-in function names that should not be shadowed by voice/instrument/wave definitions.
fn is_reserved_name(name: &str) -> bool {
    matches!(
        name,
        "sine" | "saw" | "triangle" | "square" | "pulse"
            | "noise" | "white" | "pink" | "brown"
            | "lowpass" | "highpass" | "bandpass" | "allpass"
            | "decay" | "swell"
            | "reverb" | "delay" | "chorus" | "distort" | "vibrato"
            | "compress" | "crush" | "decimate" | "degrade"
            | "lfo" | "eq" | "gain" | "loudness"
            | "arp" | "chord" | "bus" | "sidechain"
            | "noise_gate" | "bit_crush" | "pan" | "excite"
    )
}

/// A scheduled playback event with absolute sample positions.
struct ScheduledEvent {
    start_sample: u64,
    end_sample: u64,
    duration_secs: f64,
    net: Net,
    /// Optional swell envelope applied in the render loop (attack_secs, release_secs).
    /// Handled here instead of in the DSP graph for precise, non-cycling timing.
    swell: Option<(f64, f64)>,
    /// Per-event gain multiplier (used by arp for swell-based per-step dynamics).
    /// Applied multiplicatively with the swell envelope.
    gain: f32,
    /// Provenance: pattern/voice name that produced this event (for verbose output).
    source: Option<String>,
    /// Whether this event has been logged in verbose mode.
    logged: bool,
    /// Bus tag: this event contributes to the named bus envelope.
    bus_name: Option<String>,
    /// Sidechain config: duck this event's output based on a bus envelope.
    sidechain: Option<SidechainConfig>,
    /// Live note-off support: when Some, the event is fading out.
    /// Value is the sample position where release started.
    release_at: Option<u64>,
    /// Release duration in samples. Default ~50ms for clean fade.
    release_samples: u64,
    /// Unique ID for matching note-on to note-off (MIDI note number, or 0 for score events).
    note_id: u16,
    /// Voice label from `PlayAt` — the pre-substitution name (e.g., "pluck").
    voice_label: Option<String>,
    /// Resolved voice/instrument name after `with` substitution (e.g., "mau5_arp").
    voice_resolved: Option<String>,
}

/// The central audio engine. Manages voice definitions, scheduled events,
/// and renders audio samples.
pub struct Engine {
    pub sample_rate: f64,
    pub bpm: f64,
    voices: HashMap<String, Expr>,
    voice_kinds: HashMap<String, DefKind>,
    wavetables: HashMap<String, Vec<f64>>,
    schedule: Vec<ScheduledEvent>,
    current_sample: u64,
    pedal_windows: Vec<(u64, u64, Vec<String>)>,
    pedal_pending: Option<(u64, Vec<String>)>,
    /// Tempo map: list of (beat, bpm) pairs for mid-score tempo changes.
    /// Used by beats_to_samples to integrate over tempo segments.
    tempo_map: Vec<(f64, f64)>,
    /// Master bus: bandpass filter (30Hz HP + 18kHz LP) + brick-wall limiter.
    master_bus: MasterBus,
    /// Verbose mode: log event starts during rendering.
    pub verbose: bool,
    /// Bus envelopes for sidechain compression (bus_name -> current envelope level).
    bus_envelopes: HashMap<String, f32>,
    /// Per-voice level tracking (cumulative) for profile/render summary.
    voice_levels: HashMap<String, VoiceLevelStats>,
    /// Per-voice instantaneous levels for VU meter display.
    /// Updated each buffer, with ballistic decay.
    voice_vu: HashMap<String, VuMeter>,
    /// Names of voices to solo (empty = play all).
    solo_voices: Vec<String>,
}

/// Accumulated level statistics for a single voice (lifetime of render).
#[derive(Debug, Clone)]
pub struct VoiceLevelStats {
    pub sum_sq: f64,
    pub peak: f32,
    pub sample_count: u64,
    /// Per-band energy: [sub(<80Hz), low(80-300Hz), mid(300-3kHz), high(3kHz+)]
    pub band_sum_sq: [f64; 4],
    pub band_count: u64,
    /// Simple one-pole filter states for band splitting
    lp80: f64,    // lowpass at 80 Hz
    lp300: f64,   // lowpass at 300 Hz
    lp3000: f64,  // lowpass at 3000 Hz
}

impl VoiceLevelStats {
    fn new() -> Self {
        VoiceLevelStats {
            sum_sq: 0.0,
            peak: 0.0,
            sample_count: 0,
            band_sum_sq: [0.0; 4],
            band_count: 0,
            lp80: 0.0,
            lp300: 0.0,
            lp3000: 0.0,
        }
    }

    /// Accumulate a sample with band energy tracking.
    /// Uses simple one-pole filters for approximate band splitting.
    fn accumulate_with_bands(&mut self, sample: f32, sample_rate: f64) {
        let x = sample as f64;

        // One-pole lowpass coefficients: alpha = 2*pi*freq / (2*pi*freq + sample_rate)
        let a80 = (2.0 * std::f64::consts::PI * 80.0) / (2.0 * std::f64::consts::PI * 80.0 + sample_rate);
        let a300 = (2.0 * std::f64::consts::PI * 300.0) / (2.0 * std::f64::consts::PI * 300.0 + sample_rate);
        let a3000 = (2.0 * std::f64::consts::PI * 3000.0) / (2.0 * std::f64::consts::PI * 3000.0 + sample_rate);

        // Update lowpass filters
        self.lp80 += a80 * (x - self.lp80);
        self.lp300 += a300 * (x - self.lp300);
        self.lp3000 += a3000 * (x - self.lp3000);

        // Band energy: derived from filter outputs
        let sub = self.lp80;                        // <80 Hz
        let low = self.lp300 - self.lp80;           // 80-300 Hz
        let mid = self.lp3000 - self.lp300;         // 300-3000 Hz
        let high = x - self.lp3000;                 // 3000+ Hz

        self.band_sum_sq[0] += sub * sub;
        self.band_sum_sq[1] += low * low;
        self.band_sum_sq[2] += mid * mid;
        self.band_sum_sq[3] += high * high;
        self.band_count += 1;
    }

    /// Get RMS level for a frequency band in dBFS.
    pub fn band_rms_db(&self, band: usize) -> f64 {
        if self.band_count == 0 || band >= 4 {
            return -100.0;
        }
        let rms = (self.band_sum_sq[band] / self.band_count as f64).sqrt();
        if rms > 0.0 {
            20.0 * rms.log10()
        } else {
            -100.0
        }
    }

    /// RMS level in dBFS.
    pub fn rms_db(&self) -> f64 {
        if self.sample_count == 0 {
            return -100.0;
        }
        let rms = (self.sum_sq / self.sample_count as f64).sqrt();
        if rms > 0.0 {
            20.0 * rms.log10()
        } else {
            -100.0
        }
    }

    /// Peak level in dBFS.
    pub fn peak_db(&self) -> f64 {
        if self.peak > 0.0 {
            20.0 * (self.peak as f64).log10()
        } else {
            -100.0
        }
    }
}

/// Per-voice VU meter with ballistic decay and peak hold.
#[derive(Debug, Clone)]
pub struct VuMeter {
    /// Current displayed level (linear amplitude), with decay applied.
    pub level: f32,
    /// Peak hold level (linear amplitude) — stays at max for a short time.
    pub peak_hold: f32,
    /// Countdown for peak hold (in buffer cycles). When 0, peak starts decaying.
    pub peak_hold_frames: u32,
}

impl VuMeter {
    fn new() -> Self {
        VuMeter {
            level: 0.0,
            peak_hold: 0.0,
            peak_hold_frames: 0,
        }
    }

    /// Update with a new buffer's peak level. Fast attack, slow decay.
    /// Called once per audio buffer (~46ms at 2048 frames / 44100 Hz).
    fn update(&mut self, buffer_peak: f32) {
        // Fast attack: jump to new level if higher
        if buffer_peak > self.level {
            self.level = buffer_peak;
        } else {
            // Gentle decay: ~0.85 per buffer → falls ~1.4 dB per buffer
            // Takes ~15 buffers (~700ms) to drop from full to inaudible
            self.level *= 0.85;
            if self.level < 1e-5 {
                self.level = 0.0;
            }
        }

        // Peak hold: stays at max for ~1.5 seconds (~32 buffers), then decays slowly
        if buffer_peak > self.peak_hold {
            self.peak_hold = buffer_peak;
            self.peak_hold_frames = 32;
        } else if self.peak_hold_frames > 0 {
            self.peak_hold_frames -= 1;
        } else {
            self.peak_hold *= 0.92;
            if self.peak_hold < 1e-5 {
                self.peak_hold = 0.0;
            }
        }
    }

    /// Current level in dBFS.
    pub fn level_db(&self) -> f64 {
        if self.level > 0.0 {
            20.0 * (self.level as f64).log10()
        } else {
            -100.0
        }
    }

    /// Peak hold level in dBFS.
    pub fn peak_hold_db(&self) -> f64 {
        if self.peak_hold > 0.0 {
            20.0 * (self.peak_hold as f64).log10()
        } else {
            -100.0
        }
    }
}

impl Engine {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            bpm: 120.0,
            voices: HashMap::new(),
            voice_kinds: HashMap::new(),
            wavetables: HashMap::new(),
            schedule: Vec::new(),
            current_sample: 0,
            pedal_windows: Vec::new(),
            pedal_pending: None,
            tempo_map: vec![(0.0, 120.0)],
            master_bus: MasterBus::new(sample_rate),
            verbose: false,
            bus_envelopes: HashMap::new(),
            voice_levels: HashMap::new(),
            voice_vu: HashMap::new(),
            solo_voices: Vec::new(),
        }
    }

    /// Resolve VoiceRefs in an expression by inlining voice definitions.
    /// This allows extractors (bus, sidechain, swell) to find these markers
    /// even when they're defined inside a voice rather than at the play site.
    fn resolve_expr_shallow(&self, expr: &Expr) -> Expr {
        match expr {
            Expr::VoiceRef(name) => {
                if let Some(voice_expr) = self.voices.get(name) {
                    self.resolve_expr_shallow(voice_expr)
                } else {
                    expr.clone()
                }
            }
            Expr::Pipe(a, b) => Expr::Pipe(
                Box::new(self.resolve_expr_shallow(a)),
                Box::new(self.resolve_expr_shallow(b)),
            ),
            Expr::Mul(a, b) => Expr::Mul(
                Box::new(self.resolve_expr_shallow(a)),
                Box::new(self.resolve_expr_shallow(b)),
            ),
            Expr::Sum(a, b) => Expr::Sum(
                Box::new(self.resolve_expr_shallow(a)),
                Box::new(self.resolve_expr_shallow(b)),
            ),
            Expr::FnCall { name, args } => {
                // For instrument calls like piano(C4), resolve the name
                if let Some(voice_expr) = self.voices.get(name) {
                    self.resolve_expr_shallow(voice_expr)
                } else {
                    expr.clone()
                }
            }
            _ => expr.clone(),
        }
    }

    /// Get per-voice VU meter states (instantaneous with decay).
    pub fn voice_vu(&self) -> &HashMap<String, VuMeter> {
        &self.voice_vu
    }

    /// Set solo mode: only play events matching these voice labels.
    /// Pass an empty vec to disable solo (play everything).
    pub fn set_solo(&mut self, voices: Vec<String>) {
        if !voices.is_empty() {
            // Match against voice_label (alias/pattern name) OR voice_resolved
            // (the actual instrument/voice name after `with` substitution).
            self.schedule.retain(|e| {
                if let Some(ref label) = e.voice_label {
                    if voices.iter().any(|v| v == label) {
                        return true;
                    }
                }
                if let Some(ref resolved) = e.voice_resolved {
                    if voices.iter().any(|v| v == resolved) {
                        return true;
                    }
                }
                false
            });
        }
        self.solo_voices = voices;
    }

    /// Get accumulated per-voice level statistics.
    pub fn voice_levels(&self) -> &HashMap<String, VoiceLevelStats> {
        &self.voice_levels
    }

    /// Process a parsed command.
    pub fn handle_command(&mut self, cmd: Command) -> Result<()> {
        match cmd {
            Command::VoiceDef { name, expr, kind } => {
                if is_reserved_name(&name) {
                    eprintln!("Warning: '{}' is a built-in function name. Defining a {} with this name will shadow the built-in and may cause unexpected behavior.", name, kind);
                }
                self.voice_kinds.insert(name.clone(), kind);
                self.voices.insert(name, expr);
            }
            Command::WaveDef { name, samples } => {
                if is_reserved_name(&name) {
                    eprintln!("Warning: '{}' is a built-in function name. Defining a wave with this name will shadow the built-in.", name);
                }
                self.wavetables.insert(name, samples);
            }
            Command::SetBpm { bpm, at_beat } => {
                let change_beat = at_beat.unwrap_or(0.0);
                if (bpm - self.bpm).abs() > 0.001 && change_beat > 0.0 {
                    // Mid-score tempo change — add to tempo map
                    self.tempo_map.push((change_beat, bpm));
                } else if self.tempo_map.len() == 1 && change_beat == 0.0 {
                    // Initial BPM set — update the first entry
                    self.tempo_map[0] = (0.0, bpm);
                }
                self.bpm = bpm;
            }
            Command::PlayAt {
                beat,
                expr,
                duration_beats,
                source,
                voice_label,
            } => {
                let start_sample = self.beats_to_samples(beat);

                // Check for arpeggiator: arp(...) or arp(...) >> swell(...)
                if self.try_handle_arp(&expr, start_sample, duration_beats, voice_label.clone(), extract_voice_label(&expr))?.is_some() {
                    // Arpeggiator handled — sub-events already scheduled
                } else {
                    let duration_samples = self.beats_to_samples(duration_beats);
                    let end_sample = start_sample + duration_samples;
                    let duration_secs = duration_beats * 60.0 / self.bpm;
                    // Resolve top-level VoiceRefs so extractors can find
                    // bus/sidechain/swell inside voice definitions
                    let resolved = self.resolve_expr_shallow(&expr);
                    let swell = extract_swell(&resolved);
                    let bus_name = extract_bus(&resolved);
                    let sidechain = extract_sidechain(&resolved);
                    let clean_expr = strip_sidechain(&strip_bus(&strip_swell(&expr)));

                    let net =
                        build_graph(&clean_expr, &self.voices, &self.wavetables, self.sample_rate, Some(duration_secs))?;

                    self.schedule.push(ScheduledEvent {
                        start_sample,
                        end_sample,
                        duration_secs,
                        net,
                        swell,
                        gain: 1.0,
                        source,
                        logged: false,
                        bus_name,
                        sidechain,
                        release_at: None,
                        release_samples: (self.sample_rate * 0.05) as u64,
                        note_id: 0,
                        voice_label,
                        voice_resolved: extract_voice_label(&expr),
                    });
                }
            }
            Command::PedalDown { beat, voices } => {
                let sample = self.beats_to_samples(beat);
                self.pedal_pending = Some((sample, voices));
            }
            Command::PedalUp { beat, voices: _ } => {
                let up_sample = self.beats_to_samples(beat);
                if let Some((down_sample, voices)) = self.pedal_pending.take() {
                    self.pedal_windows.push((down_sample, up_sample, voices));
                }
            }
            Command::MasterCompress(params) => {
                match params.len() {
                    1 => self.master_bus.set_compress(params[0] as f32, self.sample_rate),
                    2 => self.master_bus.set_compress_params(
                        params[0] as f32, params[1] as f32, 0.010, 0.200, self.sample_rate,
                    ),
                    4 => self.master_bus.set_compress_params(
                        params[0] as f32, params[1] as f32, params[2], params[3], self.sample_rate,
                    ),
                    _ => return Err(anyhow::anyhow!(
                        "master compress: expected 1, 2, or 4 arguments"
                    )),
                }
            }
            Command::MasterCeiling(db) => {
                self.master_bus.set_ceiling(db as f32, self.sample_rate);
            }
            Command::MasterGain(db) => {
                self.master_bus.set_gain(db as f32);
            }
            Command::MasterSaturate(amount) => {
                self.master_bus.set_saturate(amount as f32);
            }
            Command::MasterCurve { low, mid, high } => {
                self.master_bus.set_curve(low as f32, mid as f32, high as f32, self.sample_rate);
            }
            Command::MasterMultiband(params) => {
                match params.len() {
                    1 => self.master_bus.set_multiband(params[0] as f32),
                    3 => self.master_bus.set_multiband_per_band(params[0] as f32, params[1] as f32, params[2] as f32),
                    _ => eprintln!("Warning: master multiband expects 1 or 3 values"),
                }
            }
            Command::MasterCurvePreset(name) => {
                if !self.master_bus.set_curve_preset(&name, self.sample_rate) {
                    eprintln!("Warning: unknown master curve preset '{}'. Options: car, broadcast, bright, warm, flat", name);
                }
            }
            Command::Normalize { name, target } => {
                self.normalize_instrument(&name, target)?;
            }
            // Swing/humanize/with are consumed by the expander — ignore if they reach engine
            Command::SetSwing(_) | Command::SetHumanize(_) | Command::SetWith(_) => {}
            // These variants are resolved before reaching the engine
            Command::Import { .. }
            | Command::PatternDef { .. }
            | Command::SectionDef { .. }
            | Command::PlaySequential { .. }
            | Command::RepeatBlock { .. } => {
                return Err(anyhow::anyhow!(
                    "Unexpanded command reached engine — run expand_script first"
                ));
            }
        }
        Ok(())
    }

    /// Schedule an event relative to the current playback position.
    /// Used in streaming mode where "at 0" means "now".
    pub fn handle_command_relative(&mut self, cmd: Command) -> Result<()> {
        match cmd {
            Command::PlayAt {
                beat,
                expr,
                duration_beats,
                source,
                voice_label,
            } => {
                let offset = self.beats_to_samples(beat);
                let start_sample = self.current_sample + offset;

                if let Some(_) = self.try_handle_arp(&expr, start_sample, duration_beats, voice_label.clone(), extract_voice_label(&expr))? {
                    // Arpeggiator handled
                } else {
                    let duration_samples = self.beats_to_samples(duration_beats);
                    let end_sample = start_sample + duration_samples;
                    let duration_secs = duration_beats * 60.0 / self.bpm;
                    // Resolve top-level VoiceRefs so extractors can find
                    // bus/sidechain/swell inside voice definitions
                    let resolved = self.resolve_expr_shallow(&expr);
                    let swell = extract_swell(&resolved);
                    let bus_name = extract_bus(&resolved);
                    let sidechain = extract_sidechain(&resolved);
                    let clean_expr = strip_sidechain(&strip_bus(&strip_swell(&expr)));

                    let net =
                        build_graph(&clean_expr, &self.voices, &self.wavetables, self.sample_rate, Some(duration_secs))?;

                    self.schedule.push(ScheduledEvent {
                        start_sample,
                        end_sample,
                        duration_secs,
                        net,
                        swell,
                        gain: 1.0,
                        source,
                        logged: false,
                        bus_name,
                        sidechain,
                        release_at: None,
                        release_samples: (self.sample_rate * 0.05) as u64,
                        note_id: 0,
                        voice_label,
                        voice_resolved: extract_voice_label(&expr),
                    });
                }
            }
            other => self.handle_command(other)?,
        }
        Ok(())
    }

    /// Schedule a live note that plays indefinitely until release_note() is called.
    /// Used by piano mode for MIDI input where duration is unknown at note-on time.
    /// `note_id` is typically the MIDI note number, used to match note-off to note-on.
    pub fn play_live_note(&mut self, expr: Expr, note_id: u16) -> Result<()> {
        // Schedule for 30 seconds max (safety limit, release_note will end it sooner)
        let duration_beats = 30.0 * self.bpm / 60.0;
        let duration_secs = 30.0;
        let start_sample = self.current_sample;
        let end_sample = start_sample + (duration_secs * self.sample_rate) as u64;

        let net = build_graph(&expr, &self.voices, &self.wavetables, self.sample_rate, Some(duration_secs))?;

        self.schedule.push(ScheduledEvent {
            start_sample,
            end_sample,
            duration_secs,
            net,
            swell: None,
            gain: 1.0,
            source: None,
            logged: false,
            bus_name: None,
            sidechain: None,
            release_at: None,
            release_samples: (self.sample_rate * 0.08) as u64,
            note_id,
            voice_label: None,
            voice_resolved: None,
        });

        Ok(())
    }

    /// Apply sustain pedal windows to scheduled events.
    /// Notes whose end_sample falls within a pedal window get extended to the pedal-up sample.
    /// Call this after all commands have been processed but before rendering.
    pub fn apply_pedal(&mut self) {
        if self.pedal_windows.is_empty() {
            return;
        }
        for event in &mut self.schedule {
            for (down, up, voices) in &self.pedal_windows {
                // If voices is non-empty, only apply pedal to events with a matching voice_label.
                if !voices.is_empty() {
                    match &event.voice_label {
                        Some(label) if voices.contains(label) => {}
                        _ => continue,
                    }
                }
                if *down <= event.end_sample && event.end_sample <= *up {
                    event.end_sample = *up;
                    break;
                }
            }
        }
    }

    /// Render audio samples into the output buffer (mono f32).
    /// This is the hot path — called from both WAV rendering and cpal callbacks.
    pub fn render_samples(&mut self, left: &mut [f32], right: &mut [f32]) {
        // Zero both buffers
        for sample in left.iter_mut() {
            *sample = 0.0;
        }
        for sample in right.iter_mut() {
            *sample = 0.0;
        }

        let buf_start = self.current_sample;
        let buf_end = buf_start + left.len() as u64;

        // Verbose logging: print events that start in this buffer
        if self.verbose {
            let sr = self.sample_rate;
            let tempo_map = &self.tempo_map;
            for event in self.schedule.iter_mut() {
                if !event.logged && event.start_sample >= buf_start && event.start_sample < buf_end {
                    if let Some(ref src) = event.source {
                        let beat = samples_to_beats_static(event.start_sample, sr, tempo_map);
                        eprintln!("  [{:.1}] {}", beat, src);
                    }
                    event.logged = true;
                }
            }
        }

        // Accumulate per-bus peak levels for this buffer (for sidechain in next buffer)
        let mut bus_peaks: HashMap<String, f32> = HashMap::new();
        let mut voice_buffer_peaks: HashMap<String, f32> = HashMap::new();

        for event in &mut self.schedule {
            // Skip events entirely outside this buffer window
            if event.start_sample >= buf_end || event.end_sample <= buf_start {
                continue;
            }

            // Calculate the active range within this buffer
            let active_start = if event.start_sample > buf_start {
                (event.start_sample - buf_start) as usize
            } else {
                0
            };
            let active_end = if event.end_sample < buf_end {
                (event.end_sample - buf_start) as usize
            } else {
                left.len()
            };

            let fade_samples = 256u64;

            // Compute sidechain gain reduction (uses bus envelope from previous buffer)
            let sc_gain = if let Some(ref sc) = event.sidechain {
                let env_db = *self.bus_envelopes.get(&sc.bus_name).unwrap_or(&-100.0);
                if env_db > sc.threshold_db {
                    let over = env_db - sc.threshold_db;
                    let reduced = over * (1.0 - 1.0 / sc.ratio);
                    10.0_f32.powf(-reduced / 20.0)
                } else {
                    1.0
                }
            } else {
                1.0
            };

            for i in active_start..active_end {
                let pos = buf_start + i as u64;
                let samples_into = pos - event.start_sample;
                let samples_remaining = event.end_sample - pos;

                // Short fade-in/fade-out to avoid clicks at voice boundaries
                let anti_click = if samples_into < fade_samples {
                    samples_into as f32 / fade_samples as f32
                } else if samples_remaining < fade_samples {
                    samples_remaining as f32 / fade_samples as f32
                } else {
                    1.0
                };

                // Swell envelope: computed from sample position for exact timing
                let swell_env = if let Some((attack, release)) = event.swell {
                    let t = samples_into as f64 / self.sample_rate;
                    let dur = event.duration_secs;
                    let fade_in = (t / attack).min(1.0);
                    let fade_start = (dur - release).max(0.0);
                    let fade_out = if t <= fade_start {
                        1.0
                    } else {
                        (1.0 - (t - fade_start) / release).max(0.0)
                    };
                    fade_in.min(fade_out) as f32
                } else {
                    1.0
                };

                // Release envelope: fade out when note-off has been received
                let release_env = if let Some(release_start) = event.release_at {
                    if pos >= release_start {
                        let into_release = (pos - release_start) as f32;
                        let total = std::cmp::max(event.release_samples, 1) as f32;
                        (1.0 - into_release / total).max(0.0)
                    } else {
                        1.0
                    }
                } else {
                    1.0
                };

                let (out_l, out_r) = event.net.get_stereo();
                // Guard: if a voice produces NaN/Inf (e.g. filter instability
                // from extreme frequencies), kill the event immediately.
                // A NaN voice never recovers — cut it off to protect the mix.
                if !out_l.is_finite() || !out_r.is_finite() {
                    event.end_sample = pos; // mark as finished
                    break; // stop processing this event for the rest of the buffer
                }
                let env = anti_click * swell_env * release_env * event.gain * sc_gain;
                let left_sample = out_l * env;
                let right_sample = out_r * env;
                left[i] += left_sample;
                right[i] += right_sample;

                // Track per-voice level stats (use max of L/R)
                let abs_peak = left_sample.abs().max(right_sample.abs());
                if let Some(ref label) = event.voice_label {
                    let stats = self.voice_levels
                        .entry(label.clone())
                        .or_insert_with(VoiceLevelStats::new);
                    stats.sum_sq += (left_sample as f64) * (left_sample as f64);
                    if abs_peak > stats.peak {
                        stats.peak = abs_peak;
                    }
                    stats.sample_count += 1;
                    // Band energy tracking (use mono mix for band analysis)
                    let mono_sample = (left_sample + right_sample) * 0.5;
                    stats.accumulate_with_bands(mono_sample, self.sample_rate);

                    // Track buffer-local peak for VU meter
                    let bp = voice_buffer_peaks.entry(label.clone()).or_insert(0.0);
                    if abs_peak > *bp {
                        *bp = abs_peak;
                    }
                }

                // Track bus peak level
                if let Some(ref bus) = event.bus_name {
                    let peak = bus_peaks.entry(bus.clone()).or_insert(0.0);
                    if abs_peak > *peak {
                        *peak = abs_peak;
                    }
                }
            }
        }

        // Update bus envelopes for next buffer (convert peak to dB)
        for (name, peak) in bus_peaks {
            let db = if peak > 0.0 { 20.0 * peak.log10() } else { -100.0 };
            self.bus_envelopes.insert(name, db);
        }

        // Update VU meters: voices that played this buffer get their peak,
        // voices that didn't play get a decay update.
        // Use a noise floor: anything below -80 dBFS (~0.0001) is treated as silence.
        let noise_floor: f32 = 0.0001;
        let known_voices: Vec<String> = self.voice_vu.keys().cloned().collect();
        for name in &known_voices {
            let buffer_peak = voice_buffer_peaks.get(name).copied().unwrap_or(0.0);
            if buffer_peak < noise_floor {
                // Voice is silent or below noise floor — decay
                self.voice_vu.entry(name.clone()).and_modify(|vu| vu.update(0.0));
            }
        }
        for (name, peak) in voice_buffer_peaks {
            if peak >= noise_floor {
                self.voice_vu.entry(name).or_insert_with(VuMeter::new).update(peak);
            }
        }

        // Master bus: bandpass + compressor + limiter (stereo)
        self.master_bus.process_stereo(left, right);

        self.current_sample = buf_end;

        // Remove finished events
        self.schedule.retain(|e| e.end_sample > buf_end);
    }

    /// Skip ahead to a given beat position. Advances `current_sample` and
    /// drops events that end before that point. Events that span the skip
    /// point will start playing from the middle.
    pub fn skip_to_beat(&mut self, beat: f64) {
        let target_sample = self.beats_to_samples(beat);
        self.current_sample = target_sample;
        self.schedule.retain(|e| e.end_sample > target_sample);
    }

    /// Returns true when all scheduled events have finished playing.
    pub fn is_finished(&self) -> bool {
        self.schedule.is_empty()
    }

    /// Trigger note-off for a live note. Starts the release fade on all events
    /// matching the given note_id. Safe to call if no matching events exist.
    pub fn release_note(&mut self, note_id: u16) {
        let current = self.current_sample;
        let release_dur = (self.sample_rate * 0.08) as u64; // 80ms release fade
        for event in &mut self.schedule {
            if event.note_id == note_id && event.release_at.is_none() {
                event.release_at = Some(current);
                event.release_samples = release_dur;
                // Set end_sample to current + release, so the event gets cleaned up
                // after the fade completes
                event.end_sample = current + release_dur;
            }
        }
    }

    /// Set master bus compression amount (0.0 = off, 1.0 = default, 2.0 = heavy).
    pub fn set_master_compress(&mut self, amount: f32) {
        self.master_bus.set_compress(amount, self.sample_rate);
    }

    /// Set master bus compression with explicit parameters.
    pub fn set_master_compress_params(&mut self, threshold: f32, ratio: f32, attack: f64, release: f64) {
        self.master_bus.set_compress_params(threshold, ratio, attack, release, self.sample_rate);
    }

    /// Set limiter ceiling in dBFS (e.g., -0.3, -1.0).
    pub fn set_master_ceiling(&mut self, db: f32) {
        self.master_bus.set_ceiling(db, self.sample_rate);
    }

    /// Normalize an instrument to a target level (0.0-1.0 scale).
    /// Renders short test tones at multiple frequencies through the instrument,
    /// measures the average RMS, and stores a correction gain so future notes
    /// through this instrument produce output at the target level.
    fn normalize_instrument(&mut self, name: &str, target: f64) -> anyhow::Result<()> {
        use crate::engine::graph::{build_graph, substitute_var};

        // Look up the instrument definition
        let expr = self.voices.get(name)
            .ok_or_else(|| anyhow::anyhow!("normalize: unknown voice/instrument '{name}'"))?
            .clone();

        let kind = self.voice_kinds.get(name).copied();
        let is_instrument = kind == Some(DefKind::Instrument);

        // Test frequencies across the musical range
        let test_freqs: &[f64] = if is_instrument {
            &[65.41, 130.81, 261.63, 523.25, 1046.50] // C2, C3, C4, C5, C6
        } else {
            &[261.63] // Voices are fixed — just test once
        };

        let test_duration = 0.5; // seconds per test tone (long enough for decay instruments)
        let test_samples = (test_duration * self.sample_rate) as usize;
        // Window size for peak RMS measurement (~50ms) — captures the attack portion
        // which is what humans perceive as the instrument's loudness
        let window_size = (0.05 * self.sample_rate) as usize;
        let mut best_window_rms = 0.0_f64;

        for &freq in test_freqs {
            // For instruments, substitute freq variable
            let test_expr = if is_instrument {
                substitute_var(&expr, "freq", freq)
            } else {
                expr.clone()
            };

            // Build the DSP graph
            let mut net = match build_graph(
                &test_expr,
                &self.voices,
                &self.wavetables,
                self.sample_rate,
                Some(test_duration),
            ) {
                Ok(n) => n,
                Err(_) => continue, // skip frequencies that fail to build
            };
            net.set_sample_rate(self.sample_rate);
            net.allocate();

            // Render all test samples into a buffer
            let mut samples = Vec::with_capacity(test_samples);
            for _ in 0..test_samples {
                let out = net.get_mono();
                samples.push(if out.is_finite() { out as f64 } else { 0.0 });
            }

            // Find the loudest window (sliding window peak RMS)
            // This captures the attack portion rather than averaging over
            // the full duration including decay tail
            if samples.len() >= window_size {
                let mut window_sum_sq: f64 = samples[..window_size].iter().map(|s| s * s).sum();
                let mut max_sum_sq = window_sum_sq;

                for i in 1..=(samples.len() - window_size) {
                    // Slide: remove old sample, add new sample
                    let old = samples[i - 1];
                    let new = samples[i + window_size - 1];
                    window_sum_sq += new * new - old * old;
                    if window_sum_sq > max_sum_sq {
                        max_sum_sq = window_sum_sq;
                    }
                }

                let window_rms = (max_sum_sq / window_size as f64).sqrt();
                if window_rms > best_window_rms {
                    best_window_rms = window_rms;
                }
            }
        }

        if best_window_rms < 1e-10 {
            eprintln!("  normalize {name}: instrument is silent — skipping");
            return Ok(());
        }

        let measured_rms = best_window_rms;

        // Target RMS: the target level (0.0-1.0) maps to linear amplitude.
        // 1.0 = 0 dBFS, 0.5 = -6 dB, 0.25 = -12 dB, etc.
        // Floor at 1e-8 (~-160 dBFS) to prevent divide-by-zero.
        // Ambient textures may legitimately need very low levels (e.g., 0.0001 = -80 dB).
        let target_rms = target.max(1e-8).min(1.0);
        let correction = target_rms / measured_rms;

        // Apply correction: wrap the instrument's expression in a gain multiplier
        let corrected = Expr::Mul(
            Box::new(expr),
            Box::new(Expr::Number(correction)),
        );
        self.voices.insert(name.to_string(), corrected);

        let measured_db = if measured_rms > 0.0 { 20.0 * measured_rms.log10() } else { -100.0 };
        let target_db = 20.0 * target_rms.log10();
        let correction_db = 20.0 * correction.log10();
        eprintln!(
            "  normalize {name}: measured {measured_db:.1} dB, target {target_db:.1} dB, correction {correction_db:+.1} dB"
        );

        Ok(())
    }

    /// Flush the master bus limiter lookahead buffer (stereo).
    /// Call after all render_samples() calls to get the final tail samples.
    pub fn flush_master(&mut self) -> (Vec<f32>, Vec<f32>) {
        let mut left_tail = Vec::new();
        let mut right_tail = Vec::new();
        self.master_bus.flush_stereo(&mut left_tail, &mut right_tail);
        (left_tail, right_tail)
    }

    /// Get the current playback position in samples.
    pub fn current_sample(&self) -> u64 {
        self.current_sample
    }

    /// Check if the expression contains an `arp(...)` call anywhere in a pipe chain.
    /// If so, decompose it into sub-events and schedule them. Returns Ok(Some(())) if handled.
    ///
    /// Supports the pipe syntax: `voice >> arp(C4, E4, G4, 4) >> lowpass(800, 0.7)`
    /// - Everything before arp is the voice template (frequency gets substituted per note)
    /// - Everything after arp is the post-processing chain (applied to each note)
    /// - If no voice before arp, defaults to `triangle(freq) >> decay(8)`
    fn try_handle_arp(
        &mut self,
        expr: &Expr,
        base_start: u64,
        duration_beats: f64,
        voice_label: Option<String>,
        voice_resolved: Option<String>,
    ) -> Result<Option<()>> {
        // Find arp(...) in the pipe chain and split into pre/post
        let (pre_chain, arp_args, post_chain) = match extract_arp(expr) {
            Some(parts) => parts,
            None => return Ok(None),
        };

        if arp_args.len() < 2 {
            return Err(anyhow::anyhow!(
                "arp requires at least one note and a rate"
            ));
        }

        // Parse arp args: NOTES..., RATE, OPTIONS...
        // Scan backwards to find the rate (first Number or Range from the end,
        // skipping option keywords and their values).
        let opts = parse_arp_options(&arp_args)?;

        // Expand note args into frequencies
        let mut base_notes: Vec<f64> = Vec::new();
        for arg in &arp_args[..opts.notes_end] {
            match arg {
                Expr::Number(v) => base_notes.push(*v),
                Expr::VoiceRef(name) => {
                    if let Some(chord_notes) = resolve_chord(name) {
                        base_notes.extend(chord_notes);
                    } else {
                        return Err(anyhow::anyhow!(
                            "arp: '{name}' is not a known chord or note"
                        ));
                    }
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "arp: arguments must be notes or chord names"
                    ))
                }
            }
        }

        if base_notes.is_empty() {
            return Err(anyhow::anyhow!("arp: no notes provided"));
        }

        // Apply octave spanning: duplicate notes at +12, +24... semitones
        let mut spanned_notes = base_notes.clone();
        for oct in 1..opts.octaves {
            for &note in &base_notes {
                let midi = 69.0 + 12.0 * (note / 440.0).log2();
                let shifted_midi = midi + (oct as f64 * 12.0);
                let shifted_hz = 440.0 * 2.0_f64.powf((shifted_midi - 69.0) / 12.0);
                spanned_notes.push(shifted_hz);
            }
        }

        // Apply direction to build the note sequence
        let sequence = build_arp_sequence(&spanned_notes, &opts.direction);

        // Extract swell from the post-chain if present
        let swell = post_chain.as_ref().and_then(|pc| extract_swell(pc));
        let clean_post = post_chain.map(|pc| strip_swell(&pc));
        let clean_post = clean_post.and_then(|pc| {
            if matches!(&pc, Expr::FnCall { name, .. } if name == "swell") {
                None
            } else {
                Some(pc)
            }
        });

        // Speed ramp: accumulate beat positions with variable rate
        let rate_start = opts.rate_start;
        let rate_end = opts.rate_end.unwrap_or(rate_start);
        let has_ramp = opts.rate_end.is_some();

        // Estimate total steps from average rate (or exact for fixed rate)
        let avg_rate = (rate_start + rate_end) / 2.0;
        let total_steps = (duration_beats * avg_rate).round() as usize;

        // For random direction, we need an RNG per cycle
        let mut rng_state: u64 = 42; // deterministic seed

        // Pre-compute beat offsets for each step (handles speed ramp)
        let mut beat_offsets: Vec<f64> = Vec::with_capacity(total_steps);
        let mut cursor_beat = 0.0;
        for i in 0..total_steps {
            if cursor_beat >= duration_beats {
                break;
            }
            beat_offsets.push(cursor_beat);

            // Interpolate rate at this step
            let frac = if total_steps > 1 { i as f64 / (total_steps - 1) as f64 } else { 0.0 };
            let current_rate = if has_ramp {
                rate_start + (rate_end - rate_start) * frac
            } else {
                rate_start
            };
            cursor_beat += 1.0 / current_rate;
        }

        let actual_steps = beat_offsets.len();

        // Track the note index separately (skipped steps don't advance note)
        let mut note_idx: usize = 0;
        let seq_len = sequence.len();

        for i in 0..actual_steps {
            // Step pattern: check if this step should sound
            if let Some(ref pattern) = opts.step_pattern {
                if !pattern[i % pattern.len()] {
                    continue; // silent step, don't advance note_idx
                }
            }

            // Get frequency from sequence (or randomize)
            let freq = if opts.direction == ArpDirection::Random {
                // Simple deterministic pseudo-random using step index
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let rand_idx = (rng_state >> 33) as usize % sequence.len();
                sequence[rand_idx]
            } else {
                sequence[note_idx % seq_len]
            };
            note_idx += 1;

            let beat_offset = beat_offsets[i];
            let start = base_start + self.beats_to_samples(beat_offset);

            // Gate: control note duration relative to step
            let step_beats = if i + 1 < actual_steps {
                beat_offsets[i + 1] - beat_offset
            } else {
                duration_beats - beat_offset
            };
            let note_beats = step_beats * opts.gate;
            let note_samples = self.beats_to_samples(note_beats);
            let end = start + note_samples;
            let dur_secs = note_beats * 60.0 / self.bpm;

            // Build the note expression
            let note_expr = if let Some(ref voice_template) = pre_chain {
                if contains_freq_var(voice_template, &self.voices) {
                    substitute_var(voice_template, "freq", freq)
                } else {
                    let wt_names: Vec<String> = self.wavetables.keys().cloned().collect();
                    substitute_freq(voice_template, &self.voices, &wt_names, freq)
                }
            } else {
                Expr::Pipe(
                    Box::new(Expr::FnCall {
                        name: "triangle".to_string(),
                        args: vec![Expr::Number(freq)],
                    }),
                    Box::new(Expr::FnCall {
                        name: "decay".to_string(),
                        args: vec![Expr::Number(8.0)],
                    }),
                )
            };

            let full_expr = if let Some(ref post) = clean_post {
                Expr::Pipe(Box::new(note_expr), Box::new(post.clone()))
            } else {
                note_expr
            };

            let net = build_graph(&full_expr, &self.voices, &self.wavetables, self.sample_rate, Some(dur_secs))?;

            // Swell envelope gain
            let sub_swell_gain = swell.map(|(attack, release)| {
                let t_start = beat_offset * 60.0 / self.bpm;
                let total_dur = duration_beats * 60.0 / self.bpm;
                let fade_in = (t_start / attack).min(1.0);
                let fade_start = (total_dur - release).max(0.0);
                let t_end = t_start + dur_secs;
                let fade_out = if t_end <= fade_start {
                    1.0
                } else {
                    (1.0 - (t_end - fade_start) / release).max(0.0)
                };
                fade_in.min(fade_out)
            });

            // Accent: boost every Nth step
            let accent_gain = if let Some(n) = opts.accent_every {
                if i % n == 0 { 1.5 } else { 0.7 }
            } else {
                1.0
            };

            let step_gain = sub_swell_gain.unwrap_or(1.0) as f32 * accent_gain as f32;

            self.schedule.push(ScheduledEvent {
                start_sample: start,
                end_sample: end,
                duration_secs: dur_secs,
                net,
                swell: None,
                gain: step_gain,
                source: None,
                logged: false,
                bus_name: None,
                sidechain: None,
                release_at: None,
                release_samples: (self.sample_rate * 0.05) as u64,
                note_id: 0,
                voice_label: voice_label.clone(),
                voice_resolved: voice_resolved.clone(),
            });
        }

        Ok(Some(()))
    }

    /// Convert an absolute beat position to an absolute sample position.
    ///
    /// Integrates over the tempo map: each segment [start_beat, next_start_beat)
    /// contributes time at its BPM. For beat positions beyond the last tempo change,
    /// the final BPM is used.
    ///
    /// With a single BPM (no mid-score changes), this is equivalent to:
    ///   beats * 60.0 / bpm * sample_rate
    fn beats_to_samples(&self, beat: f64) -> u64 {
        let mut total_seconds = 0.0;

        for i in 0..self.tempo_map.len() {
            let (seg_start, seg_bpm) = self.tempo_map[i];

            // Where does this segment end? At the next tempo change, or infinity.
            let seg_end = if i + 1 < self.tempo_map.len() {
                self.tempo_map[i + 1].0
            } else {
                f64::INFINITY
            };

            // Skip segments entirely before beat 0 (shouldn't happen, but defensive)
            if seg_end <= 0.0 {
                continue;
            }

            // How many beats of this segment are before `beat`?
            let effective_start = seg_start;
            let effective_end = seg_end.min(beat);

            if effective_end <= effective_start {
                // This segment is entirely at or after `beat` — we're done
                // (or it's a zero-length segment)
                if seg_start >= beat {
                    break;
                }
                continue;
            }

            let beats_in_seg = effective_end - effective_start;
            total_seconds += beats_in_seg * 60.0 / seg_bpm;
        }

        (total_seconds * self.sample_rate) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beats_to_samples_single_bpm() {
        let mut engine = Engine::new(44100.0);
        engine.handle_command(Command::SetBpm { bpm: 120.0, at_beat: None }).unwrap();
        // At 120 BPM: 1 beat = 0.5 seconds = 22050 samples
        assert_eq!(engine.beats_to_samples(1.0), 22050);
        assert_eq!(engine.beats_to_samples(4.0), 88200);
        assert_eq!(engine.beats_to_samples(0.0), 0);
    }

    #[test]
    fn test_beats_to_samples_tempo_change() {
        let mut engine = Engine::new(44100.0);
        // Start at 120 BPM
        engine.handle_command(Command::SetBpm { bpm: 120.0, at_beat: Some(0.0) }).unwrap();
        // At beat 4, switch to 60 BPM
        engine.handle_command(Command::SetBpm { bpm: 60.0, at_beat: Some(4.0) }).unwrap();

        // Beat 2: entirely in the 120 BPM zone
        // 2 beats at 120 BPM = 1 second = 44100 samples
        assert_eq!(engine.beats_to_samples(2.0), 44100);

        // Beat 4: right at the boundary, entirely in 120 BPM zone
        // 4 beats at 120 BPM = 2 seconds = 88200 samples
        assert_eq!(engine.beats_to_samples(4.0), 88200);

        // Beat 6: 4 beats at 120 BPM + 2 beats at 60 BPM
        // = 2 seconds + 2 seconds = 4 seconds = 176400 samples
        assert_eq!(engine.beats_to_samples(6.0), 176400);

        // Beat 8: 4 beats at 120 BPM + 4 beats at 60 BPM
        // = 2 seconds + 4 seconds = 6 seconds = 264600 samples
        assert_eq!(engine.beats_to_samples(8.0), 264600);
    }

    #[test]
    fn test_beats_to_samples_three_tempos() {
        let mut engine = Engine::new(44100.0);
        engine.handle_command(Command::SetBpm { bpm: 60.0, at_beat: Some(0.0) }).unwrap();
        engine.handle_command(Command::SetBpm { bpm: 120.0, at_beat: Some(4.0) }).unwrap();
        engine.handle_command(Command::SetBpm { bpm: 240.0, at_beat: Some(8.0) }).unwrap();

        // Beat 4: 4 beats at 60 BPM = 4 seconds
        assert_eq!(engine.beats_to_samples(4.0), 44100 * 4);

        // Beat 8: 4 beats at 60 BPM + 4 beats at 120 BPM = 4 + 2 = 6 seconds
        assert_eq!(engine.beats_to_samples(8.0), 44100 * 6);

        // Beat 12: 4 at 60 + 4 at 120 + 4 at 240 = 4 + 2 + 1 = 7 seconds
        assert_eq!(engine.beats_to_samples(12.0), 44100 * 7);
    }
}

/// Convert a sample position back to beats using a tempo map.
/// Free function to avoid borrow conflicts when iterating schedule.
fn samples_to_beats_static(sample: u64, sample_rate: f64, tempo_map: &[(f64, f64)]) -> f64 {
    let mut remaining_secs = sample as f64 / sample_rate;
    let mut beat = 0.0;
    for i in 0..tempo_map.len() {
        let (seg_start, seg_bpm) = tempo_map[i];
        let seg_end = if i + 1 < tempo_map.len() {
            tempo_map[i + 1].0
        } else {
            f64::INFINITY
        };
        let seg_duration_beats = seg_end - seg_start;
        let seg_duration_secs = seg_duration_beats * 60.0 / seg_bpm;
        if remaining_secs <= seg_duration_secs {
            beat = seg_start + remaining_secs * seg_bpm / 60.0;
            break;
        }
        remaining_secs -= seg_duration_secs;
        beat = seg_end;
    }
    beat
}

// ---------------------------------------------------------------------------
// Arp options parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum ArpDirection {
    Up,
    Down,
    UpDown,
    Random,
}

#[derive(Debug)]
struct ArpOptions {
    notes_end: usize,     // index in arp_args where notes end (exclusive)
    rate_start: f64,
    rate_end: Option<f64>, // Some for speed ramp
    direction: ArpDirection,
    octaves: u32,
    gate: f64,
    accent_every: Option<usize>,
    step_pattern: Option<Vec<bool>>,
}

/// Known option keywords for arp (identifiers that are NOT chord names).
fn is_arp_option(name: &str) -> bool {
    matches!(
        name,
        "up" | "down" | "updown" | "random" | "gate" | "accent" | "steps"
    ) || is_direction_with_octave(name).is_some()
}

/// Parse direction keywords with optional octave suffix: "up2", "down3", "updown2".
fn is_direction_with_octave(name: &str) -> Option<(ArpDirection, u32)> {
    for (prefix, dir) in &[
        ("updown", ArpDirection::UpDown),
        ("down", ArpDirection::Down),
        ("up", ArpDirection::Up),
    ] {
        if let Some(suffix) = name.strip_prefix(prefix) {
            if suffix.is_empty() {
                return Some((dir.clone(), 1));
            }
            if let Ok(oct) = suffix.parse::<u32>() {
                if oct >= 1 && oct <= 4 {
                    return Some((dir.clone(), oct));
                }
            }
        }
    }
    None
}

/// Parse arp arguments into notes boundary, rate, and options.
///
/// Format: arp(NOTES..., RATE, OPTIONS...)
/// - NOTES: Number (frequencies) or VoiceRef (chord names)
/// - RATE: Number or Range (for speed ramp)
/// - OPTIONS: direction (up/down/updown/random/up2/down3...),
///   gate + Number, accent + Number, steps + identifier
fn parse_arp_options(args: &[Expr]) -> Result<ArpOptions> {
    let mut opts = ArpOptions {
        notes_end: 0,
        rate_start: 4.0,
        rate_end: None,
        direction: ArpDirection::Up,
        octaves: 1,
        gate: 1.0,
        accent_every: None,
        step_pattern: None,
    };

    // Scan backwards to find the rate. Skip option keywords and their values.
    // The rate is the first Number or Range we hit (scanning backwards past options).
    let mut rate_idx = None;
    let mut i = args.len();
    while i > 0 {
        i -= 1;
        match &args[i] {
            Expr::VoiceRef(name) if is_arp_option(name) => {
                // This is an option keyword. "gate", "accent", "steps" consume
                // the next arg (already skipped by the caller below).
                continue;
            }
            Expr::Number(_) | Expr::Range(_, _) => {
                // Check if this number is a value for gate/accent (preceded by keyword)
                if i > 0 {
                    if let Expr::VoiceRef(prev) = &args[i - 1] {
                        if prev == "gate" || prev == "accent" {
                            // This number is a parameter value, not the rate
                            i -= 1; // skip the keyword too
                            continue;
                        }
                    }
                }
                // This is the rate
                rate_idx = Some(i);
                break;
            }
            _ => {
                // VoiceRef that's not an option keyword — could be a chord name.
                // If we haven't found rate yet, keep scanning.
                // But if it's before any options, it's a note.
                // Actually if we encounter a non-option VoiceRef while scanning
                // backwards, we've passed all options and this is in the notes zone.
                // The rate must be right after this.
                // Check the next element (i+1):
                if i + 1 < args.len() {
                    if let Expr::Number(_) | Expr::Range(_, _) = &args[i + 1] {
                        rate_idx = Some(i + 1);
                        break;
                    }
                }
            }
        }
    }

    let rate_idx = rate_idx.ok_or_else(|| anyhow::anyhow!("arp: could not find rate argument"))?;

    // Extract rate
    match &args[rate_idx] {
        Expr::Number(v) => opts.rate_start = *v,
        Expr::Range(start, end) => {
            opts.rate_start = *start;
            opts.rate_end = Some(*end);
        }
        _ => unreachable!(),
    }

    opts.notes_end = rate_idx;

    // Parse options (everything after rate)
    let option_args = &args[rate_idx + 1..];
    let mut oi = 0;
    while oi < option_args.len() {
        match &option_args[oi] {
            Expr::VoiceRef(name) => {
                let name = name.as_str();
                if name == "random" {
                    opts.direction = ArpDirection::Random;
                } else if let Some((dir, oct)) = is_direction_with_octave(name) {
                    opts.direction = dir;
                    opts.octaves = oct;
                } else if name == "gate" {
                    oi += 1;
                    if let Some(Expr::Number(v)) = option_args.get(oi) {
                        opts.gate = *v;
                    } else {
                        return Err(anyhow::anyhow!("arp: 'gate' requires a number"));
                    }
                } else if name == "accent" {
                    oi += 1;
                    if let Some(Expr::Number(v)) = option_args.get(oi) {
                        opts.accent_every = Some(*v as usize);
                    } else {
                        return Err(anyhow::anyhow!("arp: 'accent' requires a number"));
                    }
                } else if name == "steps" {
                    oi += 1;
                    if let Some(Expr::VoiceRef(pattern)) = option_args.get(oi) {
                        let parsed: Vec<bool> = pattern
                            .chars()
                            .filter_map(|c| match c {
                                'x' | 'X' => Some(true),
                                '_' | '.' => Some(false),
                                _ => None,
                            })
                            .collect();
                        if parsed.is_empty() {
                            return Err(anyhow::anyhow!(
                                "arp: 'steps' pattern must contain x (play) and _ (rest)"
                            ));
                        }
                        opts.step_pattern = Some(parsed);
                    } else {
                        return Err(anyhow::anyhow!(
                            "arp: 'steps' requires a pattern like x_x_xx_x"
                        ));
                    }
                } else {
                    return Err(anyhow::anyhow!(
                        "arp: unknown option '{name}'"
                    ));
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "arp: unexpected argument after rate"
                ));
            }
        }
        oi += 1;
    }

    Ok(opts)
}

/// Build the note sequence after applying direction.
fn build_arp_sequence(notes: &[f64], direction: &ArpDirection) -> Vec<f64> {
    match direction {
        ArpDirection::Up => notes.to_vec(),
        ArpDirection::Down => {
            let mut seq = notes.to_vec();
            seq.reverse();
            seq
        }
        ArpDirection::UpDown => {
            if notes.len() <= 1 {
                return notes.to_vec();
            }
            // Up then down, dropping duplicates at turnaround points
            let mut seq = notes.to_vec();
            for i in (1..notes.len() - 1).rev() {
                seq.push(notes[i]);
            }
            seq
        }
        ArpDirection::Random => {
            // For random, the sequence is just the base notes —
            // randomization happens per-step in the scheduling loop.
            notes.to_vec()
        }
    }
}

/// Check if an expression tree contains VoiceRef("freq"), either directly
/// or inside a voice that it references. Used to decide whether arp should
/// use substitute_var (for instruments) or substitute_freq (for plain voices).
fn contains_freq_var(expr: &Expr, voices: &HashMap<String, Expr>) -> bool {
    match expr {
        Expr::VoiceRef(name) if name == "freq" => true,
        Expr::VoiceRef(name) => {
            if let Some(voice_expr) = voices.get(name) {
                contains_freq_var(voice_expr, voices)
            } else {
                false
            }
        }
        Expr::FnCall { args, .. } => args.iter().any(|a| contains_freq_var(a, voices)),
        Expr::Pipe(a, b) | Expr::Sum(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
            contains_freq_var(a, voices) || contains_freq_var(b, voices)
        }
        Expr::Number(_) | Expr::Range(_, _) => false,
    }
}
