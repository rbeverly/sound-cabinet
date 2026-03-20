use std::collections::HashMap;

use anyhow::Result;
use fundsp::hacker::*;

use crate::dsl::ast::{Command, DefKind, Expr};
use crate::dsl::parser::resolve_chord;
use crate::engine::effects::MasterBus;
use crate::engine::graph::{build_graph, extract_arp, extract_bus, extract_sidechain, extract_swell, strip_bus, strip_sidechain, strip_swell, substitute_freq, substitute_var, SidechainConfig};

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
    pedal_windows: Vec<(u64, u64)>,
    pedal_pending: Option<u64>,
    /// Tempo map: list of (beat, bpm) pairs for mid-score tempo changes.
    /// Used by beats_to_samples to integrate over tempo segments.
    tempo_map: Vec<(f64, f64)>,
    /// Master bus: bandpass filter (30Hz HP + 18kHz LP) + brick-wall limiter.
    master_bus: MasterBus,
    /// Verbose mode: log event starts during rendering.
    pub verbose: bool,
    /// Bus envelopes for sidechain compression (bus_name -> current envelope level).
    bus_envelopes: HashMap<String, f32>,
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
        }
    }

    /// Process a parsed command.
    pub fn handle_command(&mut self, cmd: Command) -> Result<()> {
        match cmd {
            Command::VoiceDef { name, expr, kind } => {
                self.voice_kinds.insert(name.clone(), kind);
                self.voices.insert(name, expr);
            }
            Command::WaveDef { name, samples } => {
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
            } => {
                let start_sample = self.beats_to_samples(beat);

                // Check for arpeggiator: arp(...) or arp(...) >> swell(...)
                if self.try_handle_arp(&expr, start_sample, duration_beats)?.is_some() {
                    // Arpeggiator handled — sub-events already scheduled
                } else {
                    let duration_samples = self.beats_to_samples(duration_beats);
                    let end_sample = start_sample + duration_samples;
                    let duration_secs = duration_beats * 60.0 / self.bpm;
                    let swell = extract_swell(&expr);
                    let bus_name = extract_bus(&expr);
                    let sidechain = extract_sidechain(&expr);
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
                    });
                }
            }
            Command::PedalDown { beat } => {
                let sample = self.beats_to_samples(beat);
                self.pedal_pending = Some(sample);
            }
            Command::PedalUp { beat } => {
                let up_sample = self.beats_to_samples(beat);
                if let Some(down_sample) = self.pedal_pending.take() {
                    self.pedal_windows.push((down_sample, up_sample));
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
            } => {
                let offset = self.beats_to_samples(beat);
                let start_sample = self.current_sample + offset;

                if let Some(_) = self.try_handle_arp(&expr, start_sample, duration_beats)? {
                    // Arpeggiator handled
                } else {
                    let duration_samples = self.beats_to_samples(duration_beats);
                    let end_sample = start_sample + duration_samples;
                    let duration_secs = duration_beats * 60.0 / self.bpm;
                    let swell = extract_swell(&expr);
                    let bus_name = extract_bus(&expr);
                    let sidechain = extract_sidechain(&expr);
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
                    });
                }
            }
            other => self.handle_command(other)?,
        }
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
            for &(down, up) in &self.pedal_windows {
                if down <= event.end_sample && event.end_sample <= up {
                    event.end_sample = up;
                    break;
                }
            }
        }
    }

    /// Render audio samples into the output buffer (mono f32).
    /// This is the hot path — called from both WAV rendering and cpal callbacks.
    pub fn render_samples(&mut self, buffer: &mut [f32]) {
        // Zero the buffer
        for sample in buffer.iter_mut() {
            *sample = 0.0;
        }

        let buf_start = self.current_sample;
        let buf_end = buf_start + buffer.len() as u64;

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
                buffer.len()
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

                let out = event.net.get_mono();
                let sample = out * anti_click * swell_env * event.gain * sc_gain;
                buffer[i] += sample;

                // Track bus peak level
                if let Some(ref bus) = event.bus_name {
                    let abs = sample.abs();
                    let peak = bus_peaks.entry(bus.clone()).or_insert(0.0);
                    if abs > *peak {
                        *peak = abs;
                    }
                }
            }
        }

        // Update bus envelopes for next buffer (convert peak to dB)
        for (name, peak) in bus_peaks {
            let db = if peak > 0.0 { 20.0 * peak.log10() } else { -100.0 };
            self.bus_envelopes.insert(name, db);
        }

        // Master bus: bandpass + limiter
        self.master_bus.process(buffer);

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

    /// Flush the master bus limiter lookahead buffer.
    /// Call after all render_samples() calls to get the final tail samples.
    pub fn flush_master(&mut self) -> Vec<f32> {
        let mut tail = Vec::new();
        self.master_bus.flush(&mut tail);
        tail
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

        // Last arg is rate (notes per beat), preceding args are frequencies
        let rate = match arp_args.last() {
            Some(Expr::Number(v)) => *v,
            _ => {
                return Err(anyhow::anyhow!(
                    "arp: last argument (rate) must be a number"
                ))
            }
        };
        // Expand arp note args — supports individual notes (Expr::Number from note names)
        // and chord names (Expr::VoiceRef that resolves as a chord like "Cm7")
        let mut notes: Vec<f64> = Vec::new();
        for arg in &arp_args[..arp_args.len() - 1] {
            match arg {
                Expr::Number(v) => notes.push(*v),
                Expr::VoiceRef(name) => {
                    if let Some(chord_notes) = resolve_chord(name) {
                        notes.extend(chord_notes);
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

        // Extract swell from the post-chain if present
        let swell = post_chain.as_ref().and_then(|pc| extract_swell(pc));
        let clean_post = post_chain.map(|pc| strip_swell(&pc));
        // Drop the post-chain if it was only swell (strip_swell returns the inner expr)
        let clean_post = clean_post.and_then(|pc| {
            // If stripping swell left us with a pass-through equivalent, discard it
            if matches!(&pc, Expr::FnCall { name, .. } if name == "swell") {
                None
            } else {
                Some(pc)
            }
        });

        let step_beats = 1.0 / rate;
        let step_samples = self.beats_to_samples(step_beats);
        let total_steps = (duration_beats * rate).round() as usize;

        for i in 0..total_steps {
            let freq = notes[i % notes.len()];
            let start = base_start + (i as u64) * step_samples;
            let end = start + step_samples;
            let dur_secs = step_beats * 60.0 / self.bpm;

            // Build the note expression: substitute freq into voice template,
            // or use default triangle+decay if no voice was provided.
            // Use substitute_var for instrument templates (contain VoiceRef("freq")),
            // substitute_freq for plain voices (only replaces oscillator args).
            let note_expr = if let Some(ref voice_template) = pre_chain {
                if contains_freq_var(voice_template, &self.voices) {
                    substitute_var(voice_template, "freq", freq)
                } else {
                    let wt_names: Vec<String> = self.wavetables.keys().cloned().collect();
                    substitute_freq(voice_template, &self.voices, &wt_names, freq)
                }
            } else {
                // Default voice: triangle oscillator with decay
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

            // Pipe through the post-processing chain if present
            let full_expr = if let Some(ref post) = clean_post {
                Expr::Pipe(Box::new(note_expr), Box::new(post.clone()))
            } else {
                note_expr
            };

            let net = build_graph(&full_expr, &self.voices, &self.wavetables, self.sample_rate, Some(dur_secs))?;

            // Compute per-step swell envelope value and bake it as a gain multiplier
            let sub_swell_gain = swell.map(|(attack, release)| {
                let t_start = (i as f64) * step_beats * 60.0 / self.bpm;
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

            // Per-step gain from swell envelope (if present)
            let step_gain = sub_swell_gain.unwrap_or(1.0) as f32;

            self.schedule.push(ScheduledEvent {
                start_sample: start,
                end_sample: end,
                duration_secs: dur_secs,
                net,
                swell: None,
                gain: step_gain,
                source: None, // arp sub-events don't carry individual provenance
                logged: false,
                bus_name: None,
                sidechain: None,
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
