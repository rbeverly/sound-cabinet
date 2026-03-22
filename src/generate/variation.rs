//! Variation generator: produces multiple distinct versions of a resolved phrase.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use super::pattern::PatternFile;
use super::resolver::{self, GenerateParams, ResolvedNote};
use super::theory::Pitch;
use anyhow::Result;

/// Generate N variations of a pattern.
///
/// Variation 0 is always the "straight" resolution (no modifications).
/// Subsequent variations introduce controlled randomness:
/// - Starting degree offset (root → 3rd or 5th)
/// - Octave displacement of individual notes
/// - Interval stretching (step → small leap)
pub fn generate_variations(
    pattern: &PatternFile,
    params: &GenerateParams,
    count: usize,
    base_seed: u64,
) -> Result<Vec<Vec<ResolvedNote>>> {
    let mut results = Vec::with_capacity(count);

    // Variation 0: straight resolution
    let base = resolver::resolve_pattern(pattern, params)?;
    results.push(base.clone());

    // Additional variations with randomness
    for vi in 1..count {
        let mut rng = StdRng::seed_from_u64(base_seed.wrapping_add(vi as u64));
        let varied = vary_phrase(&base, params, &mut rng);
        results.push(varied);
    }

    Ok(results)
}

/// Apply random variations to a resolved phrase.
fn vary_phrase(
    base: &[ResolvedNote],
    params: &GenerateParams,
    rng: &mut StdRng,
) -> Vec<ResolvedNote> {
    let mut notes: Vec<ResolvedNote> = base.to_vec();

    for note in &mut notes {
        if note.is_rest {
            continue;
        }

        // 30% chance: offset starting pitch by a scale interval
        if rng.gen::<f64>() < 0.30 {
            let offsets = [-2, -1, 2, 3, 4]; // diatonic steps
            let offset = offsets[rng.gen_range(0..offsets.len())];
            let new_pitch = params.scale.leap(note.pitch, offset);
            let clamped = params.range.clamp(new_pitch);
            note.pitch = clamped;
        }

        // 15% chance: octave displacement
        if rng.gen::<f64>() < 0.15 {
            let direction = if rng.gen_bool(0.5) { 12 } else { -12 };
            let candidate = Pitch::from_midi(note.pitch.midi() + direction);
            if params.range.contains(candidate) {
                note.pitch = candidate;
            }
        }

        // Ensure still in range
        note.pitch = params.range.clamp(note.pitch);
    }

    notes
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::pattern::PatternFile;
    use crate::generate::theory::*;

    fn test_params() -> GenerateParams {
        GenerateParams {
            scale: Scale::new(PitchClass::C, Mode::Major),
            chords: vec![Chord::parse("Cmaj").unwrap()],
            range: PitchRange::parse("C2-C4").unwrap(),
            voice_name: "bass".to_string(),
            time_sig: (4, 4),
        }
    }

    fn test_pattern() -> PatternFile {
        PatternFile::from_yaml(
            r#"
name: Test
type: bass
time: "4/4"
rhythm:
  hits: ["1/4", "1/4", "1/4", "1/4"]
contour: [root, step_up, step_up, step_down]
emphasis: [strong, weak, weak, medium]
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_generates_correct_count() {
        let pattern = test_pattern();
        let params = test_params();
        let variations = generate_variations(&pattern, &params, 5, 42).unwrap();
        assert_eq!(variations.len(), 5);
    }

    #[test]
    fn test_same_seed_same_output() {
        let pattern = test_pattern();
        let params = test_params();
        let a = generate_variations(&pattern, &params, 3, 42).unwrap();
        let b = generate_variations(&pattern, &params, 3, 42).unwrap();

        for (va, vb) in a.iter().zip(b.iter()) {
            for (na, nb) in va.iter().zip(vb.iter()) {
                assert_eq!(na.pitch.midi(), nb.pitch.midi());
            }
        }
    }

    #[test]
    fn test_different_seeds_different_output() {
        let pattern = test_pattern();
        let params = test_params();
        let a = generate_variations(&pattern, &params, 5, 42).unwrap();
        let b = generate_variations(&pattern, &params, 5, 99).unwrap();

        // At least some variations should differ (comparing variation 1+)
        let mut any_differ = false;
        for vi in 1..5 {
            for (na, nb) in a[vi].iter().zip(b[vi].iter()) {
                if na.pitch.midi() != nb.pitch.midi() {
                    any_differ = true;
                }
            }
        }
        assert!(any_differ, "Different seeds should produce different variations");
    }

    #[test]
    fn test_all_in_range() {
        let pattern = test_pattern();
        let params = test_params();
        let variations = generate_variations(&pattern, &params, 10, 42).unwrap();

        for (vi, var) in variations.iter().enumerate() {
            for note in var {
                if !note.is_rest {
                    assert!(
                        params.range.contains(note.pitch),
                        "Variation {}: note {} out of range",
                        vi,
                        note.pitch.to_note_name(None)
                    );
                }
            }
        }
    }

    #[test]
    fn test_variation_0_is_unmodified() {
        let pattern = test_pattern();
        let params = test_params();
        let variations = generate_variations(&pattern, &params, 3, 42).unwrap();
        let base = resolver::resolve_pattern(&pattern, &params).unwrap();

        // Variation 0 should match the base resolution exactly
        for (a, b) in variations[0].iter().zip(base.iter()) {
            assert_eq!(a.pitch.midi(), b.pitch.midi());
        }
    }
}
