use std::collections::HashMap;

use anyhow::Result;
use fundsp::hacker::*;

use crate::dsl::ast::{Command, Expr};
use crate::engine::graph::build_graph;

/// A scheduled playback event with absolute sample positions.
struct ScheduledEvent {
    start_sample: u64,
    end_sample: u64,
    net: Net,
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
                let duration_samples = self.beats_to_samples(duration_beats);
                let end_sample = start_sample + duration_samples;

                let net = build_graph(&expr, &self.voices, self.sample_rate)?;

                self.schedule.push(ScheduledEvent {
                    start_sample,
                    end_sample,
                    net,
                });
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
                let duration_samples = self.beats_to_samples(duration_beats);
                let end_sample = start_sample + duration_samples;

                let net = build_graph(&expr, &self.voices, self.sample_rate)?;

                self.schedule.push(ScheduledEvent {
                    start_sample,
                    end_sample,
                    net,
                });
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

        for event in &mut self.schedule {
            for (i, sample) in buffer.iter_mut().enumerate() {
                let pos = self.current_sample + i as u64;

                if pos >= event.start_sample && pos < event.end_sample {
                    // Apply a short fade-in/fade-out envelope to avoid clicks
                    let samples_into = pos - event.start_sample;
                    let samples_remaining = event.end_sample - pos;
                    let fade_samples = 256u64;
                    let env = if samples_into < fade_samples {
                        samples_into as f32 / fade_samples as f32
                    } else if samples_remaining < fade_samples {
                        samples_remaining as f32 / fade_samples as f32
                    } else {
                        1.0
                    };

                    let out = event.net.get_mono();
                    *sample += out * env;
                }
            }
        }

        self.current_sample += buffer.len() as u64;

        // Remove finished events
        self.schedule
            .retain(|e| e.end_sample > self.current_sample);
    }

    /// Returns true when all scheduled events have finished playing.
    pub fn is_finished(&self) -> bool {
        self.schedule.is_empty()
    }

    /// Get the current playback position in samples.
    pub fn current_sample(&self) -> u64 {
        self.current_sample
    }

    fn beats_to_samples(&self, beats: f64) -> u64 {
        let seconds_per_beat = 60.0 / self.bpm;
        (beats * seconds_per_beat * self.sample_rate) as u64
    }
}
