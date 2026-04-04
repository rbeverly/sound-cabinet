//! Extract musical note events from expanded .sc commands.
//!
//! Walks the flat list of expanded PlayAt commands and extracts voice names,
//! pitches (Hz→MIDI), durations, and timing for sheet music export.

use crate::dsl::ast::{Command, Expr};
use crate::generate::theory::Pitch;

/// A note event extracted from a PlayAt command.
#[derive(Debug, Clone)]
pub struct NoteEvent {
    /// Absolute beat position in the score.
    pub beat: f64,
    /// Pitch, if a pitched note. None for percussion/fixed voices.
    pub pitch: Option<Pitch>,
    /// Duration in beats.
    pub duration_beats: f64,
    /// Voice or instrument name (e.g., "piano", "bass", "kick").
    pub voice_name: String,
    /// The as-played voice label, preserved across `with` substitution.
    /// E.g., "duelingpiano1" even if it resolves to "piano".
    pub voice_label: Option<String>,
    /// Gain/velocity multiplier extracted from the expression.
    pub velocity: f64,
    /// Provenance: which pattern/section produced this event.
    pub source: Option<String>,
}

/// Tempo change event.
#[derive(Debug, Clone)]
pub struct TempoEvent {
    pub beat: f64,
    pub bpm: f64,
}

/// Pedal event.
#[derive(Debug, Clone)]
pub struct PedalEvent {
    pub beat: f64,
    pub down: bool,
}

/// Result of extracting notes from an expanded script.
pub struct ExtractedScore {
    pub notes: Vec<NoteEvent>,
    pub tempos: Vec<TempoEvent>,
    pub pedals: Vec<PedalEvent>,
}

/// Extract all musical events from expanded commands.
pub fn extract_notes(commands: &[Command]) -> ExtractedScore {
    let mut notes = Vec::new();
    let mut tempos = Vec::new();
    let mut pedals = Vec::new();

    for cmd in commands {
        match cmd {
            Command::PlayAt {
                beat,
                expr,
                duration_beats,
                source,
                voice_label,
                velocity,
            } => {
                let (voice_name, freq_hz, gain) = extract_voice_and_freq(expr);
                let pitch = freq_hz.map(Pitch::from_hz);

                notes.push(NoteEvent {
                    beat: *beat,
                    pitch,
                    duration_beats: *duration_beats,
                    voice_name,
                    voice_label: voice_label.clone(),
                    velocity: gain,
                    source: source.clone(),
                });
            }
            Command::SetBpm { bpm, at_beat } => {
                tempos.push(TempoEvent {
                    beat: at_beat.unwrap_or(0.0),
                    bpm: *bpm,
                });
            }
            Command::PedalDown { beat, .. } => {
                pedals.push(PedalEvent {
                    beat: *beat,
                    down: true,
                });
            }
            Command::PedalUp { beat, .. } => {
                pedals.push(PedalEvent {
                    beat: *beat,
                    down: false,
                });
            }
            _ => {}
        }
    }

    // Sort notes by beat, then by voice name for stable ordering
    notes.sort_by(|a, b| {
        a.beat
            .partial_cmp(&b.beat)
            .unwrap()
            .then(a.voice_name.cmp(&b.voice_name))
    });

    tempos.sort_by(|a, b| a.beat.partial_cmp(&b.beat).unwrap());

    ExtractedScore {
        notes,
        tempos,
        pedals,
    }
}

/// Walk an expression tree to extract the voice/instrument name,
/// frequency (if any), and gain multiplier.
///
/// Returns `(voice_name, optional_freq_hz, gain)`.
fn extract_voice_and_freq(expr: &Expr) -> (String, Option<f64>, f64) {
    match expr {
        // Direct function call: `piano(261.63)` or `sine(440)`
        Expr::FnCall { name, args } => {
            let freq = args.first().and_then(|a| match a {
                Expr::Number(v) => Some(*v),
                _ => None,
            });
            (name.clone(), freq, 1.0)
        }

        // Voice reference (fixed voice, no freq): `kick`
        Expr::VoiceRef(name) => (name.clone(), None, 1.0),

        // Pipe chain: `piano(C4) >> lowpass(...)` — the source is the left side
        Expr::Pipe(left, _right) => extract_voice_and_freq(left),

        // Gain multiplier: `piano(C4) * 0.7` or `0.7 * piano(C4)`
        Expr::Mul(left, right) => {
            // One side is usually a Number (the gain), the other is the voice
            match (left.as_ref(), right.as_ref()) {
                (Expr::Number(g), other) | (other, Expr::Number(g)) => {
                    let (name, freq, inner_gain) = extract_voice_and_freq_inner(other);
                    (name, freq, inner_gain * g)
                }
                _ => {
                    // Both sides are complex — try left first
                    let (name, freq, gain) = extract_voice_and_freq(left);
                    if name != "_unknown" {
                        (name, freq, gain)
                    } else {
                        extract_voice_and_freq(right)
                    }
                }
            }
        }

        // Sum: `0.5 * sine(440) + 0.3 * saw(440)` — take the first voice
        Expr::Sum(left, _right) => extract_voice_and_freq(left),

        // Number alone (e.g., `dc(0.5)` as a control signal)
        Expr::Number(_) => ("_unknown".to_string(), None, 1.0),

        // Other
        _ => ("_unknown".to_string(), None, 1.0),
    }
}

/// Inner helper that handles the non-gain side of a Mul.
fn extract_voice_and_freq_inner(expr: &Expr) -> (String, Option<f64>, f64) {
    extract_voice_and_freq(expr)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::ast::Expr;

    #[test]
    fn test_extract_fn_call() {
        let expr = Expr::FnCall {
            name: "piano".to_string(),
            args: vec![Expr::Number(261.63)],
        };
        let (name, freq, gain) = extract_voice_and_freq(&expr);
        assert_eq!(name, "piano");
        assert!((freq.unwrap() - 261.63).abs() < 0.01);
        assert!((gain - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_extract_voice_ref() {
        let expr = Expr::VoiceRef("kick".to_string());
        let (name, freq, _) = extract_voice_and_freq(&expr);
        assert_eq!(name, "kick");
        assert!(freq.is_none());
    }

    #[test]
    fn test_extract_pipe_chain() {
        // piano(440) >> lowpass(2000, 0.7)
        let expr = Expr::Pipe(
            Box::new(Expr::FnCall {
                name: "piano".to_string(),
                args: vec![Expr::Number(440.0)],
            }),
            Box::new(Expr::FnCall {
                name: "lowpass".to_string(),
                args: vec![Expr::Number(2000.0), Expr::Number(0.7)],
            }),
        );
        let (name, freq, _) = extract_voice_and_freq(&expr);
        assert_eq!(name, "piano");
        assert!((freq.unwrap() - 440.0).abs() < 0.01);
    }

    #[test]
    fn test_extract_gain_multiply() {
        // piano(440) * 0.5
        let expr = Expr::Mul(
            Box::new(Expr::FnCall {
                name: "piano".to_string(),
                args: vec![Expr::Number(440.0)],
            }),
            Box::new(Expr::Number(0.5)),
        );
        let (name, freq, gain) = extract_voice_and_freq(&expr);
        assert_eq!(name, "piano");
        assert!((freq.unwrap() - 440.0).abs() < 0.01);
        assert!((gain - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_extract_commands() {
        let commands = vec![
            Command::SetBpm {
                bpm: 120.0,
                at_beat: None,
            },
            Command::PlayAt {
                beat: 0.0,
                expr: Expr::FnCall {
                    name: "piano".to_string(),
                    args: vec![Expr::Number(261.63)],
                },
                duration_beats: 1.0,
                source: Some("verse_a".to_string()),
                voice_label: None,
            },
            Command::PlayAt {
                beat: 1.0,
                expr: Expr::VoiceRef("kick".to_string()),
                duration_beats: 0.5,
                source: Some("drums_a".to_string()),
                voice_label: None,
            },
        ];

        let extracted = extract_notes(&commands);
        assert_eq!(extracted.notes.len(), 2);
        assert_eq!(extracted.tempos.len(), 1);
        assert_eq!(extracted.tempos[0].bpm, 120.0);

        assert_eq!(extracted.notes[0].voice_name, "piano");
        assert!(extracted.notes[0].pitch.is_some());

        assert_eq!(extracted.notes[1].voice_name, "kick");
        assert!(extracted.notes[1].pitch.is_none());
    }

    #[test]
    fn test_hz_to_pitch_a4() {
        let p = Pitch::from_hz(440.0);
        assert_eq!(p.midi(), 69); // A4
    }

    #[test]
    fn test_hz_to_pitch_c4() {
        let p = Pitch::from_hz(261.63);
        assert_eq!(p.midi(), 60); // C4
    }
}
