use std::collections::HashMap;

use anyhow::Result;
use fundsp::hacker::*;

use crate::dsl::ast::{Command, Expr};
use crate::dsl::parser::resolve_chord;
use crate::engine::graph::{build_graph, extract_arp, extract_swell, strip_swell, substitute_freq};

/// A scheduled playback event with absolute sample positions.
struct ScheduledEvent {
    start_sample: u64,
    end_sample: u64,
    duration_secs: f64,
    net: Net,
    /// Optional swell envelope applied in the render loop (attack_secs, release_secs).
    /// Handled here instead of in the DSP graph for precise, non-cycling timing.
    swell: Option<(f64, f64)>,
}

/// The central audio engine. Manages voice definitions, scheduled events,
/// and renders audio samples.
pub struct Engine {
    pub sample_rate: f64,
    pub bpm: f64,
    voices: HashMap<String, Expr>,
    schedule: Vec<ScheduledEvent>,
    current_sample: u64,
}

impl Engine {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            bpm: 120.0,
            voices: HashMap::new(),
            schedule: Vec::new(),
            current_sample: 0,
        }
    }

    /// Process a parsed command.
    pub fn handle_command(&mut self, cmd: Command) -> Result<()> {
        match cmd {
            Command::VoiceDef { name, expr } => {
                self.voices.insert(name, expr);
            }
            Command::SetBpm(bpm) => {
                self.bpm = bpm;
            }
            Command::PlayAt {
                beat,
                expr,
                duration_beats,
            } => {
                let start_sample = self.beats_to_samples(beat);

                // Check for arpeggiator: arp(...) or arp(...) >> swell(...)
                if let Some(arp_result) = self.try_handle_arp(&expr, start_sample, duration_beats)? {
                    // Arpeggiator handled — sub-events already scheduled
                    let _ = arp_result;
                } else {
                    let duration_samples = self.beats_to_samples(duration_beats);
                    let end_sample = start_sample + duration_samples;
                    let duration_secs = duration_beats * 60.0 / self.bpm;
                    let swell = extract_swell(&expr);
                    let clean_expr = strip_swell(&expr);

                    let net =
                        build_graph(&clean_expr, &self.voices, self.sample_rate, Some(duration_secs))?;

                    self.schedule.push(ScheduledEvent {
                        start_sample,
                        end_sample,
                        duration_secs,
                        net,
                        swell,
                    });
                }
            }
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
                    let clean_expr = strip_swell(&expr);

                    let net =
                        build_graph(&clean_expr, &self.voices, self.sample_rate, Some(duration_secs))?;

                    self.schedule.push(ScheduledEvent {
                        start_sample,
                        end_sample,
                        duration_secs,
                        net,
                        swell,
                    });
                }
            }
            other => self.handle_command(other)?,
        }
        Ok(())
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
                buffer[i] += out * anti_click * swell_env;
            }
        }

        self.current_sample = buf_end;

        // Remove finished events
        self.schedule.retain(|e| e.end_sample > buf_end);
    }

    /// Returns true when all scheduled events have finished playing.
    pub fn is_finished(&self) -> bool {
        self.schedule.is_empty()
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
            // or use default triangle+decay if no voice was provided
            let note_expr = if let Some(ref voice_template) = pre_chain {
                substitute_freq(voice_template, &self.voices, freq)
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

            let net = build_graph(&full_expr, &self.voices, self.sample_rate, Some(dur_secs))?;

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

            self.schedule.push(ScheduledEvent {
                start_sample: start,
                end_sample: end,
                duration_secs: dur_secs,
                net,
                swell: None,
            });

            // Apply the pre-computed swell gain to this sub-event
            if let Some(env_val) = sub_swell_gain {
                if env_val < 1.0 {
                    if let Some(event) = self.schedule.last_mut() {
                        let gain_net = build_graph(
                            &Expr::Number(env_val),
                            &self.voices,
                            self.sample_rate,
                            None,
                        )?;
                        let original = std::mem::replace(
                            &mut event.net,
                            build_graph(
                                &Expr::Number(0.0),
                                &self.voices,
                                self.sample_rate,
                                None,
                            )?,
                        );
                        event.net = original * gain_net;
                    }
                }
            }
        }

        Ok(Some(()))
    }

    fn beats_to_samples(&self, beats: f64) -> u64 {
        let seconds_per_beat = 60.0 / self.bpm;
        (beats * seconds_per_beat * self.sample_rate) as u64
    }
}
