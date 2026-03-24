//! Percussion pattern expander: converts drum YAML into .sc patterns.
//!
//! Drum patterns use the same rhythm/emphasis notation as melodic patterns
//! but operate on fixed voices (kick, snare, hat) rather than pitched instruments.

use anyhow::Result;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use super::pattern::{emphasis_to_velocity, DrumPattern, DrumVoice};
use super::rhythm::{self, RhythmHit};
use super::writer;
use super::theory::PitchClass;

/// A resolved drum hit ready for output.
#[derive(Debug, Clone)]
pub struct DrumHit {
    pub voice: String,
    pub pitch: String,
    pub beat_offset: f64,
    pub duration_beats: f64,
    pub velocity: f64,
}

/// Expand a drum pattern into resolved hits.
pub fn expand_drum_pattern(pattern: &DrumPattern) -> Result<Vec<DrumHit>> {
    let mut hits = Vec::new();

    for dv in &pattern.voices {
        let parsed = rhythm::parse_rhythm(&dv.rhythm)?;

        for (i, hit) in parsed.hits.iter().enumerate() {
            if hit.is_rest() {
                continue;
            }

            let velocity = if !dv.emphasis.is_empty() {
                emphasis_to_velocity(&dv.emphasis[i])
            } else {
                // Default: strong on downbeats, medium elsewhere
                if (parsed.offsets[i] % 1.0).abs() < 0.01 {
                    1.0
                } else {
                    0.7
                }
            };

            hits.push(DrumHit {
                voice: dv.voice.clone(),
                pitch: dv.pitch.clone(),
                beat_offset: parsed.offsets[i],
                duration_beats: hit.duration().min(0.5), // drums are short
                velocity,
            });
        }
    }

    // Sort by beat offset for clean output
    hits.sort_by(|a, b| a.beat_offset.partial_cmp(&b.beat_offset).unwrap());

    Ok(hits)
}

/// Generate N variations of a drum pattern.
pub fn generate_drum_variations(
    pattern: &DrumPattern,
    count: usize,
    base_seed: u64,
) -> Result<Vec<Vec<DrumHit>>> {
    let base = expand_drum_pattern(pattern)?;
    let mut results = vec![base.clone()];

    for vi in 1..count {
        let mut rng = StdRng::seed_from_u64(base_seed.wrapping_add(vi as u64));
        let varied = vary_drums(&base, &pattern.voices, &mut rng);
        results.push(varied);
    }

    Ok(results)
}

/// Apply drum-specific variations: ghost notes and displacement.
fn vary_drums(base: &[DrumHit], voices: &[DrumVoice], rng: &mut StdRng) -> Vec<DrumHit> {
    let mut hits = base.to_vec();

    // Slight velocity variation on existing hits
    for hit in &mut hits {
        if rng.gen::<f64>() < 0.2 {
            hit.velocity = (hit.velocity * rng.gen_range(0.7..1.1)).clamp(0.1, 1.0);
        }
    }

    // Add ghost notes on hats/snare (20% chance per rest position)
    for dv in voices {
        if dv.voice == "kick" {
            continue; // don't ghost the kick
        }
        let parsed = rhythm::parse_rhythm(&dv.rhythm).unwrap_or_else(|_| {
            rhythm::ParsedRhythm {
                hits: vec![],
                offsets: vec![],
                total_beats: 0.0,
            }
        });
        for (i, hit) in parsed.hits.iter().enumerate() {
            if hit.is_rest() && rng.gen::<f64>() < 0.15 {
                hits.push(DrumHit {
                    voice: dv.voice.clone(),
                    pitch: dv.pitch.clone(),
                    beat_offset: parsed.offsets[i],
                    duration_beats: 0.25,
                    velocity: 0.2, // ghost
                });
            }
        }
    }

    hits.sort_by(|a, b| a.beat_offset.partial_cmp(&b.beat_offset).unwrap());
    hits
}

/// Write drum variations as .sc patterns.
pub fn write_drum_sc(
    variations: &[Vec<DrumHit>],
    pattern_name: &str,
    total_beats: f64,
) -> String {
    let mut out = String::new();

    out.push_str(&format!("// Generated drums: {}\n\n", pattern_name));

    for (i, hits) in variations.iter().enumerate() {
        let letter = writer::variation_letter_pub(i);
        let var_name = format!("drums_{}", letter);

        out.push_str(&format!(
            "pattern {} = {} beats\n",
            var_name,
            writer::format_beats_pub(total_beats)
        ));

        for hit in hits {
            let beat_str = writer::format_beats_pub(hit.beat_offset);
            let dur_str = writer::format_beats_pub(hit.duration_beats);

            if (hit.velocity - 1.0).abs() < 0.01 {
                out.push_str(&format!(
                    "  at {beat_str} play {}({}) for {dur_str} beats\n",
                    hit.voice, hit.pitch
                ));
            } else {
                out.push_str(&format!(
                    "  at {beat_str} play {}({}) * {vel} for {dur_str} beats\n",
                    hit.voice, hit.pitch,
                    vel = writer::format_velocity_pub(hit.velocity)
                ));
            }
        }
        out.push('\n');
    }

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_rock_beat() {
        let yaml = r#"
name: Basic Rock
time: "4/4"
voices:
  - voice: kick
    pitch: A1
    rhythm: ["1/4", "~/4", "1/4", "~/4"]
    emphasis: [strong, "~", strong, "~"]
  - voice: snare
    pitch: G3
    rhythm: ["~/4", "1/4", "~/4", "1/4"]
    emphasis: ["~", strong, "~", strong]
  - voice: hat
    pitch: C5
    rhythm: ["1/8", "1/8", "1/8", "1/8", "1/8", "1/8", "1/8", "1/8"]
    emphasis: [strong, weak, medium, weak, strong, weak, medium, weak]
"#;
        let pattern = DrumPattern::from_yaml(yaml).unwrap();
        let hits = expand_drum_pattern(&pattern).unwrap();

        // Should have 2 kicks + 2 snares + 8 hats = 12 hits
        assert_eq!(hits.len(), 12);

        // First hit should be at beat 0
        assert!((hits[0].beat_offset - 0.0).abs() < 0.01);

        // Kicks at beats 0, 2
        let kicks: Vec<_> = hits.iter().filter(|h| h.voice == "kick").collect();
        assert_eq!(kicks.len(), 2);
        assert!((kicks[0].beat_offset - 0.0).abs() < 0.01);
        assert!((kicks[1].beat_offset - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_drum_variations_deterministic() {
        let yaml = r#"
name: Test
time: "4/4"
voices:
  - voice: kick
    pitch: A1
    rhythm: ["1/4", "~/4", "1/4", "~/4"]
  - voice: hat
    pitch: C5
    rhythm: ["1/8", "1/8", "1/8", "1/8", "1/8", "1/8", "1/8", "1/8"]
"#;
        let pattern = DrumPattern::from_yaml(yaml).unwrap();
        let a = generate_drum_variations(&pattern, 3, 42).unwrap();
        let b = generate_drum_variations(&pattern, 3, 42).unwrap();

        assert_eq!(a.len(), b.len());
        for (va, vb) in a[0].iter().zip(b[0].iter()) {
            assert_eq!(va.voice, vb.voice);
            assert!((va.beat_offset - vb.beat_offset).abs() < 0.01);
        }
    }

    #[test]
    fn test_variation_0_unchanged() {
        let yaml = r#"
name: Test
time: "4/4"
voices:
  - voice: kick
    pitch: A1
    rhythm: ["1/4", "~/4", "1/4", "~/4"]
"#;
        let pattern = DrumPattern::from_yaml(yaml).unwrap();
        let base = expand_drum_pattern(&pattern).unwrap();
        let variations = generate_drum_variations(&pattern, 3, 42).unwrap();

        // Variation 0 = base
        assert_eq!(variations[0].len(), base.len());
        for (a, b) in variations[0].iter().zip(base.iter()) {
            assert_eq!(a.voice, b.voice);
            assert!((a.beat_offset - b.beat_offset).abs() < 0.01);
        }
    }
}
