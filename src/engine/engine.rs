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
    /// Live note-off support: when Some, the event is fading out.
    /// Value is the sample position where release started.
    release_at: Option<u64>,
    /// Release duration in samples. Default ~50ms for clean fade.
    release_samples: u64,
    /// Unique ID for matching note-on to note-off (MIDI note number, or 0 for score events).
    note_id: u16,
    /// Voice label from `PlayAt`, used by pedal voice filtering.
    voice_label: Option<String>,
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
                voice_label,
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
                        release_at: None,
                        release_samples: (self.sample_rate * 0.05) as u64,
                        note_id: 0,
                        voice_label,
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
                        release_at: None,
                        release_samples: (self.sample_rate * 0.05) as u64,
                        note_id: 0,
                        voice_label,
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

                let out = event.net.get_mono();
                // Guard: if a voice produces NaN/Inf (e.g. filter instability
                // from extreme frequencies), kill the event immediately.
                // A NaN voice never recovers — cut it off to protect the mix.
                if !out.is_finite() {
                    event.end_sample = pos; // mark as finished
                    break; // stop processing this event for the rest of the buffer
                }
                let sample = out * anti_click * swell_env * release_env * event.gain * sc_gain;
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
                voice_label: None,
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
                                '.' => Some(false),
                                _ => None,
                            })
                            .collect();
                        if parsed.is_empty() {
                            return Err(anyhow::anyhow!(
                                "arp: 'steps' pattern must contain x (play) and . (rest)"
                            ));
                        }
                        opts.step_pattern = Some(parsed);
                    } else {
                        return Err(anyhow::anyhow!(
                            "arp: 'steps' requires a pattern like x.x.xx.x"
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
