//! Contour resolver: maps abstract contour tokens to concrete pitches
//! given a scale, chord progression, and instrument range.

use anyhow::{anyhow, Result};

use super::pattern::{emphasis_to_velocity, PatternFile};
use super::rhythm;
use super::theory::{Chord, Pitch, PitchRange, Scale};

/// Parameters for a generation pass.
pub struct GenerateParams {
    pub scale: Scale,
    pub chords: Vec<Chord>,
    pub range: PitchRange,
    pub voice_name: String,
    pub time_sig: (u32, u32),
}

/// A fully resolved note ready for output.
#[derive(Debug, Clone)]
pub struct ResolvedNote {
    pub beat_offset: f64,
    pub pitch: Pitch,
    pub duration_beats: f64,
    pub velocity: f64,
    pub is_rest: bool,
}

/// Resolve a pattern file into concrete notes.
pub fn resolve_pattern(
    pattern: &PatternFile,
    params: &GenerateParams,
) -> Result<Vec<ResolvedNote>> {
    let parsed_rhythm = rhythm::parse_rhythm(pattern.rhythm_hits())?;
    let bar_beats = rhythm::beats_per_bar(params.time_sig.0, params.time_sig.1);
    let contour = pattern.contour_tokens();

    let mut notes = Vec::with_capacity(parsed_rhythm.hits.len());
    let mut cursor_pitch: Option<Pitch> = None;

    for (i, (hit, contour_token)) in parsed_rhythm
        .hits
        .iter()
        .zip(contour.iter())
        .enumerate()
    {
        let beat_offset = parsed_rhythm.offsets[i];
        let duration = hit.duration();

        // Velocity from emphasis
        let velocity = if !pattern.emphasis.is_empty() {
            emphasis_to_velocity(&pattern.emphasis[i])
        } else {
            default_metric_emphasis(beat_offset, bar_beats)
        };

        if hit.is_rest() {
            notes.push(ResolvedNote {
                beat_offset,
                pitch: Pitch::from_midi(60), // placeholder, won't be used
                duration_beats: duration,
                velocity,
                is_rest: true,
            });
            continue;
        }

        // Determine active chord at this beat position
        let chord = active_chord(&params.chords, beat_offset, bar_beats);

        // Initialize cursor from chord root if this is the first note
        let effective_cursor = cursor_pitch.or_else(|| {
            Some(params.range.clamp(chord.root_pitch()))
        });

        // Resolve contour token to a pitch
        let pitch = resolve_token(
            contour_token.trim(),
            effective_cursor,
            &params.scale,
            chord,
            &params.range,
            // Look ahead for 'approach': what's the next bar's chord root?
            next_chord_root(&params.chords, beat_offset, bar_beats, &params.range),
        )?;

        let clamped = params.range.clamp(pitch);
        cursor_pitch = Some(clamped);

        notes.push(ResolvedNote {
            beat_offset,
            pitch: clamped,
            duration_beats: duration,
            velocity,
            is_rest: false,
        });
    }

    Ok(notes)
}

/// Determine which chord is active at a given beat position.
fn active_chord<'a>(chords: &'a [Chord], beat: f64, bar_beats: f64) -> &'a Chord {
    if chords.is_empty() {
        panic!("No chords provided");
    }
    if chords.len() == 1 {
        return &chords[0];
    }
    // Each chord gets equal share of bars. If pattern is 4 beats (1 bar)
    // and there are 4 chords, each chord gets 1 beat.
    // If pattern is 4 beats and there are 2 chords, each gets 2 beats.
    let _beats_per_chord = bar_beats; // 1 bar per chord by default
    // Actually: distribute chords evenly across the pattern
    // For now: one chord per bar, cycling
    let bar_index = (beat / bar_beats).floor() as usize;
    &chords[bar_index % chords.len()]
}

/// Get the root pitch of the next bar's chord (for 'approach' token).
fn next_chord_root(
    chords: &[Chord],
    beat: f64,
    bar_beats: f64,
    range: &PitchRange,
) -> Pitch {
    if chords.is_empty() {
        return Pitch::from_midi(60);
    }
    let bar_index = (beat / bar_beats).floor() as usize;
    let next_chord = &chords[(bar_index + 1) % chords.len()];
    let root = next_chord.root_pitch();
    range.clamp(root)
}

/// Resolve a single contour token to a pitch.
fn resolve_token(
    token: &str,
    cursor: Option<Pitch>,
    scale: &Scale,
    chord: &Chord,
    range: &PitchRange,
    next_root: Pitch,
) -> Result<Pitch> {
    match token {
        "root" => {
            let root = chord.root_pitch();
            Ok(range.clamp(root))
        }

        "hold" => {
            cursor.ok_or_else(|| anyhow!("'hold' used before any note"))
        }

        "step_up" => {
            let current = cursor.ok_or_else(|| anyhow!("'step_up' used before any note"))?;
            Ok(scale.step_up(current))
        }

        "step_down" => {
            let current = cursor.ok_or_else(|| anyhow!("'step_down' used before any note"))?;
            Ok(scale.step_down(current))
        }

        "half_up" => {
            let current = cursor.ok_or_else(|| anyhow!("'half_up' used before any note"))?;
            Ok(current.transpose(1))
        }

        "half_down" => {
            let current = cursor.ok_or_else(|| anyhow!("'half_down' used before any note"))?;
            Ok(current.transpose(-1))
        }

        "approach" => {
            // Chromatic half-step below the next bar's chord root
            Ok(next_root.transpose(-1))
        }

        "neighbor_up" => {
            let current = cursor.ok_or_else(|| anyhow!("'neighbor_up' used before any note"))?;
            Ok(scale.step_up(current))
        }

        "neighbor_down" => {
            let current =
                cursor.ok_or_else(|| anyhow!("'neighbor_down' used before any note"))?;
            Ok(scale.step_down(current))
        }

        "passing" => {
            // Diatonic step from current position (like step_up, contextually)
            let current = cursor.ok_or_else(|| anyhow!("'passing' used before any note"))?;
            Ok(scale.step_up(current))
        }

        "chord_low" => {
            let tones = chord.tones_in_range(range.low, range.high);
            tones
                .first()
                .copied()
                .ok_or_else(|| anyhow!("No chord tones in range for 'chord_low'"))
        }

        "chord_mid" => {
            let tones = chord.tones_in_range(range.low, range.high);
            if tones.is_empty() {
                return Err(anyhow!("No chord tones in range for 'chord_mid'"));
            }
            Ok(tones[tones.len() / 2])
        }

        "chord_high" => {
            let tones = chord.tones_in_range(range.low, range.high);
            tones
                .last()
                .copied()
                .ok_or_else(|| anyhow!("No chord tones in range for 'chord_high'"))
        }

        _ if token.starts_with("leap_up_") => {
            let steps: i32 = token
                .strip_prefix("leap_up_")
                .unwrap()
                .parse()
                .map_err(|_| anyhow!("Invalid leap: '{token}'"))?;
            let current = cursor.ok_or_else(|| anyhow!("'{token}' used before any note"))?;
            Ok(scale.leap(current, steps))
        }

        _ if token.starts_with("leap_down_") => {
            let steps: i32 = token
                .strip_prefix("leap_down_")
                .unwrap()
                .parse()
                .map_err(|_| anyhow!("Invalid leap: '{token}'"))?;
            let current = cursor.ok_or_else(|| anyhow!("'{token}' used before any note"))?;
            Ok(scale.leap(current, -steps))
        }

        _ => Err(anyhow!("Unknown contour token: '{token}'")),
    }
}

/// Default metric emphasis when no emphasis array is provided.
/// Strong on beat 1, medium on beat 3 (in 4/4), weak elsewhere.
fn default_metric_emphasis(beat_offset: f64, bar_beats: f64) -> f64 {
    let pos_in_bar = beat_offset % bar_beats;
    if (pos_in_bar - 0.0).abs() < 0.01 {
        1.0 // downbeat
    } else if (pos_in_bar - bar_beats / 2.0).abs() < 0.01 {
        0.7 // mid-bar
    } else if (pos_in_bar % 1.0).abs() < 0.01 {
        0.5 // on a beat
    } else {
        0.4 // off-beat
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::theory::{Mode, PitchClass};

    fn make_params(key: PitchClass, mode: Mode, chords: &[&str], range: &str) -> GenerateParams {
        GenerateParams {
            scale: Scale::new(key, mode),
            chords: chords
                .iter()
                .map(|c| Chord::parse(c).unwrap())
                .collect(),
            range: PitchRange::parse(range).unwrap(),
            voice_name: "bass".to_string(),
            time_sig: (4, 4),
        }
    }

    #[test]
    fn test_walking_jazz_bass() {
        let yaml = r#"
name: Walking Jazz Bass
type: bass
time: "4/4"
rhythm:
  hits: ["1/4", "1/4", "1/4", "1/4"]
contour: [root, step_up, step_up, approach]
emphasis: [strong, weak, weak, medium]
"#;
        let pattern = PatternFile::from_yaml(yaml).unwrap();
        let params = make_params(
            PitchClass::D,
            Mode::Dorian,
            &["Dm7"],
            "C2-G3",
        );

        let notes = resolve_pattern(&pattern, &params).unwrap();
        assert_eq!(notes.len(), 4);

        // Beat 0 should be D (chord root)
        assert_eq!(notes[0].pitch.pitch_class(), PitchClass::D.semitone());
        assert!(!notes[0].is_rest);

        // All notes should be in range
        for note in &notes {
            if !note.is_rest {
                assert!(params.range.contains(note.pitch),
                    "Note {} out of range", note.pitch.to_note_name(Some(PitchClass::D)));
            }
        }
    }

    #[test]
    fn test_with_rests() {
        let yaml = r#"
name: Test
type: bass
time: "4/4"
rhythm:
  hits: ["1/4", "~/4", "1/4", "~/4"]
contour: [root, "~", step_up, "~"]
"#;
        let pattern = PatternFile::from_yaml(yaml).unwrap();
        let params = make_params(PitchClass::C, Mode::Major, &["Cmaj"], "C2-G3");

        let notes = resolve_pattern(&pattern, &params).unwrap();
        assert_eq!(notes.len(), 4);
        assert!(!notes[0].is_rest);
        assert!(notes[1].is_rest);
        assert!(!notes[2].is_rest);
        assert!(notes[3].is_rest);
    }

    #[test]
    fn test_chord_tones() {
        let yaml = r#"
name: Alberti
type: accomp
time: "4/4"
rhythm:
  hits: ["1/8", "1/8", "1/8", "1/8"]
contour: [chord_low, chord_high, chord_mid, chord_high]
emphasis: [strong, weak, medium, weak]
"#;
        let pattern = PatternFile::from_yaml(yaml).unwrap();
        let params = make_params(PitchClass::C, Mode::Major, &["Cmaj"], "C3-C5");

        let notes = resolve_pattern(&pattern, &params).unwrap();
        assert_eq!(notes.len(), 4);
        // chord_low should be lower than chord_high
        assert!(notes[0].pitch.midi() < notes[1].pitch.midi());
    }

    #[test]
    fn test_approach_is_chromatic() {
        let yaml = r#"
name: Test Approach
type: bass
time: "4/4"
rhythm:
  hits: ["1/4", "1/4", "1/4", "1/4"]
contour: [root, step_up, step_up, approach]
"#;
        let pattern = PatternFile::from_yaml(yaml).unwrap();
        // Chords: Dm7 then G7. Approach on beat 3 should be half-step below G.
        let params = make_params(PitchClass::D, Mode::Dorian, &["Dm7", "G7"], "C2-G3");

        let notes = resolve_pattern(&pattern, &params).unwrap();
        // The approach note (beat 3) should be F# (half step below G)
        let approach = &notes[3];
        // next chord root is G, approach = G - 1 semitone = F#
        assert_eq!(approach.pitch.pitch_class(), 6); // F# = 6
    }
}
