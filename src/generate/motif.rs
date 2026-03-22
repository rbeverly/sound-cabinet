//! Motif expander: transforms a short motif into a multi-bar phrase
//! by applying a sequence of musical transformations.
//!
//! A motif is a short musical idea (3-6 notes). The structure is a sequence
//! of transformations like "statement", "sequence_up", "inversion", "resolve"
//! that build a cohesive melody by repeating the motif in varied forms.

use anyhow::{anyhow, Result};

use super::pattern::{MotifSpec, PatternFile, RhythmSpec};
use super::rhythm;

/// Expand a motif-based pattern into a full rhythm/contour/emphasis pattern.
/// Returns a new PatternFile with the motif expanded into direct arrays.
pub fn expand_motif(pattern: &PatternFile, time_sig: (u32, u32)) -> Result<PatternFile> {
    let motif = pattern
        .motif
        .as_ref()
        .ok_or_else(|| anyhow!("Pattern '{}' has no motif to expand", pattern.name))?;

    let bar_beats = rhythm::beats_per_bar(time_sig.0, time_sig.1);

    // Get or generate structure
    let structure = if let Some(s) = &pattern.structure {
        s.clone()
    } else {
        default_structure(pattern.complexity.as_deref())
    };

    // Compute motif duration in beats
    let motif_beats = motif_duration(&motif.rhythm)?;

    // Expand each transformation into one bar
    let mut all_rhythm: Vec<String> = Vec::new();
    let mut all_contour: Vec<String> = Vec::new();
    let mut all_emphasis: Vec<String> = Vec::new();

    // Track the last bar's output for "repeat"
    let mut last_bar_rhythm: Vec<String> = Vec::new();
    let mut last_bar_contour: Vec<String> = Vec::new();
    let mut last_bar_emphasis: Vec<String> = Vec::new();

    // Track cumulative degree offset for sequence_up/down
    let mut degree_offset: i32 = 0;

    for transform in &structure {
        let (bar_r, bar_c, bar_e) = expand_transform(
            transform.trim(),
            motif,
            bar_beats,
            motif_beats,
            &last_bar_rhythm,
            &last_bar_contour,
            &last_bar_emphasis,
            &mut degree_offset,
        )?;

        last_bar_rhythm = bar_r.clone();
        last_bar_contour = bar_c.clone();
        last_bar_emphasis = bar_e.clone();

        all_rhythm.extend(bar_r);
        all_contour.extend(bar_c);
        all_emphasis.extend(bar_e);
    }

    // Build the expanded PatternFile
    Ok(PatternFile {
        name: pattern.name.clone(),
        pattern_type: pattern.pattern_type.clone(),
        tags: pattern.tags.clone(),
        time: pattern.time.clone(),
        rhythm: Some(RhythmSpec { hits: all_rhythm }),
        contour: Some(all_contour),
        emphasis: all_emphasis,
        motif: None,
        structure: None,
        complexity: None,
        notes: pattern.notes.clone(),
    })
}

/// Default structure based on complexity level.
fn default_structure(complexity: Option<&str>) -> Vec<String> {
    match complexity.unwrap_or("simple") {
        "simple" => vec![
            "statement".into(),
            "repeat".into(),
            "sequence_up".into(),
            "resolve".into(),
        ],
        "moderate" => vec![
            "statement".into(),
            "sequence_up".into(),
            "departure".into(),
            "return".into(),
            "statement".into(),
            "sequence_down".into(),
            "extension".into(),
            "resolve".into(),
        ],
        "complex" => vec![
            "statement".into(),
            "sequence_up".into(),
            "sequence_up".into(),
            "departure_high".into(),
            "inversion".into(),
            "sequence_down".into(),
            "departure".into(),
            "extension".into(),
            "return".into(),
            "statement".into(),
            "truncation".into(),
            "resolve".into(),
        ],
        _ => default_structure(Some("simple")),
    }
}

/// Compute total beats of a motif's rhythm.
fn motif_duration(rhythm: &[String]) -> Result<f64> {
    let parsed = rhythm::parse_rhythm(rhythm)?;
    Ok(parsed.total_beats)
}

/// Expand a single transformation into one bar of rhythm/contour/emphasis.
#[allow(clippy::too_many_arguments)]
fn expand_transform(
    transform: &str,
    motif: &MotifSpec,
    bar_beats: f64,
    motif_beats: f64,
    last_rhythm: &[String],
    last_contour: &[String],
    last_emphasis: &[String],
    degree_offset: &mut i32,
) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
    match transform {
        "statement" => {
            *degree_offset = 0;
            let (r, c, e) = pad_to_bar(
                &motif.rhythm,
                &apply_degree_offset(&motif.contour, 0),
                &motif_emphasis(motif),
                bar_beats,
                motif_beats,
            );
            Ok((r, c, e))
        }

        "repeat" => {
            if last_rhythm.is_empty() {
                // No previous bar — treat as statement
                return expand_transform(
                    "statement",
                    motif,
                    bar_beats,
                    motif_beats,
                    last_rhythm,
                    last_contour,
                    last_emphasis,
                    degree_offset,
                );
            }
            Ok((
                last_rhythm.to_vec(),
                last_contour.to_vec(),
                last_emphasis.to_vec(),
            ))
        }

        "sequence_up" => {
            *degree_offset += 1;
            let contour = apply_degree_offset(&motif.contour, *degree_offset);
            let (r, c, e) =
                pad_to_bar(&motif.rhythm, &contour, &motif_emphasis(motif), bar_beats, motif_beats);
            Ok((r, c, e))
        }

        "sequence_down" => {
            *degree_offset -= 1;
            let contour = apply_degree_offset(&motif.contour, *degree_offset);
            let (r, c, e) =
                pad_to_bar(&motif.rhythm, &contour, &motif_emphasis(motif), bar_beats, motif_beats);
            Ok((r, c, e))
        }

        "inversion" => {
            let contour = invert_contour(&motif.contour);
            let contour = apply_degree_offset(&contour, *degree_offset);
            let (r, c, e) =
                pad_to_bar(&motif.rhythm, &contour, &motif_emphasis(motif), bar_beats, motif_beats);
            Ok((r, c, e))
        }

        "retrograde" => {
            let mut contour = motif.contour.clone();
            contour.reverse();
            let contour = apply_degree_offset(&contour, *degree_offset);
            let (r, c, e) =
                pad_to_bar(&motif.rhythm, &contour, &motif_emphasis(motif), bar_beats, motif_beats);
            Ok((r, c, e))
        }

        "augmentation" => {
            let rhythm: Vec<String> = motif.rhythm.iter().map(|r| augment_rhythm(r)).collect();
            let contour = apply_degree_offset(&motif.contour, *degree_offset);
            // Augmented rhythm may exceed bar — truncate
            let aug_beats = motif_duration(&rhythm).unwrap_or(bar_beats);
            let (r, c, e) =
                pad_to_bar(&rhythm, &contour, &motif_emphasis(motif), bar_beats, aug_beats);
            Ok((r, c, e))
        }

        "truncation" => {
            let n = (motif.rhythm.len() + 1) / 2; // first half
            let rhythm: Vec<String> = motif.rhythm[..n].to_vec();
            let contour: Vec<String> = motif.contour[..n].to_vec();
            let contour = apply_degree_offset(&contour, *degree_offset);
            let emph = &motif_emphasis(motif)[..n];
            let trunc_beats = motif_duration(&rhythm).unwrap_or(bar_beats / 2.0);
            let (r, c, e) =
                pad_to_bar(&rhythm, &contour, emph, bar_beats, trunc_beats);
            Ok((r, c, e))
        }

        "extension" => {
            // Motif + stepwise notes to fill bar
            let mut rhythm = motif.rhythm.clone();
            let mut contour = apply_degree_offset(&motif.contour, *degree_offset);
            let mut emph = motif_emphasis(motif).to_vec();
            let mut current_beats = motif_beats;

            let mut going_up = true;
            while current_beats + 0.5 <= bar_beats {
                rhythm.push("1/8".into());
                contour.push(if going_up {
                    "step_up".into()
                } else {
                    "step_down".into()
                });
                emph.push("weak".into());
                current_beats += 0.5;
                going_up = !going_up;
            }
            // Fill remaining with rest
            let remaining = bar_beats - current_beats;
            if remaining > 0.01 {
                let rest = format!("~/{}", (4.0 / remaining).round() as i32);
                rhythm.push(rest);
                contour.push("~".into());
                emph.push("~".into());
            }

            Ok((rhythm, contour, emph))
        }

        "departure" | "departure_high" | "departure_low" => {
            // Contrasting material — wider leaps, different rhythm
            let bias = match transform {
                "departure_high" => "up",
                "departure_low" => "down",
                _ => "mixed",
            };
            let (r, c, e) = build_departure(bar_beats, bias);
            Ok((r, c, e))
        }

        "return" => {
            // Echo first 2 notes of motif, then step down to root
            let n = motif.rhythm.len().min(2);
            let mut rhythm: Vec<String> = motif.rhythm[..n].to_vec();
            let mut contour: Vec<String> = apply_degree_offset(&motif.contour[..n], *degree_offset);
            let mut emph: Vec<String> = motif_emphasis(motif)[..n].to_vec();

            // Add step_down + root to fill
            rhythm.push("1/4".into());
            contour.push("step_down".into());
            emph.push("medium".into());
            rhythm.push("1/4".into());
            contour.push("root".into());
            emph.push("strong".into());

            let return_beats = motif_duration(&rhythm).unwrap_or(bar_beats);
            let (r, c, e) = pad_to_bar(&rhythm, &contour, &emph, bar_beats, return_beats);
            *degree_offset = 0; // reset after return
            Ok((r, c, e))
        }

        "resolve" => {
            // Stepwise descent to root
            *degree_offset = 0;
            let r = vec!["1/4".into(), "1/4".into(), "1/2".into()];
            let c = vec!["step_down".into(), "step_down".into(), "root".into()];
            let e = vec!["medium".into(), "weak".into(), "strong".into()];
            let resolve_beats = 4.0; // 1 + 1 + 2
            let (r, c, e) = pad_to_bar(&r, &c, &e, bar_beats, resolve_beats);
            Ok((r, c, e))
        }

        "approach" => {
            let r = vec!["1/4".into(), "1/4".into(), "~/2".into()];
            let c = vec!["step_down".into(), "approach".into(), "~".into()];
            let e = vec!["medium".into(), "medium".into(), "~".into()];
            Ok((r, c, e))
        }

        "rest" => {
            // Full bar of rest
            let rest_str = format!("~/{}", (4.0 / bar_beats).round().max(1.0) as i32);
            Ok((vec![rest_str], vec!["~".into()], vec!["~".into()]))
        }

        _ => Err(anyhow!("Unknown transformation: '{transform}'")),
    }
}

/// Apply a degree offset to a contour array.
/// "root" with offset 2 becomes: start 2 degrees higher.
/// We do this by replacing the first "root" with leap_up_N (or step_up repeated).
fn apply_degree_offset(contour: &[String], offset: i32) -> Vec<String> {
    if offset == 0 {
        return contour.to_vec();
    }

    let mut result = contour.to_vec();
    // Find first non-rest token and adjust it
    for token in &mut result {
        let t = token.trim();
        if t == "~" {
            continue;
        }
        if t == "root" {
            // Replace root with a leap from root
            if offset > 0 {
                *token = format!("leap_up_{offset}");
            } else {
                *token = format!("leap_down_{}", -offset);
            }
        }
        // For other tokens (step_up, etc.), they're relative so they stay the same.
        // Only the starting point shifts via the root replacement.
        break;
    }
    result
}

/// Invert contour: flip directions. step_up → step_down, leap_up_N → leap_down_N, etc.
fn invert_contour(contour: &[String]) -> Vec<String> {
    contour
        .iter()
        .map(|token| {
            let t = token.trim();
            match t {
                "step_up" => "step_down".into(),
                "step_down" => "step_up".into(),
                "half_up" => "half_down".into(),
                "half_down" => "half_up".into(),
                "neighbor_up" => "neighbor_down".into(),
                "neighbor_down" => "neighbor_up".into(),
                _ if t.starts_with("leap_up_") => t.replace("leap_up_", "leap_down_"),
                _ if t.starts_with("leap_down_") => t.replace("leap_down_", "leap_up_"),
                _ => token.clone(), // root, hold, chord_*, approach, ~, etc. stay
            }
        })
        .collect()
}

/// Double rhythm durations for augmentation.
fn augment_rhythm(rhythm_str: &str) -> String {
    let s = rhythm_str.trim();
    if s == "~" || s.starts_with("~/") {
        // Double the rest too
        if s == "~" {
            "~/2".into() // default quarter rest → half rest
        } else if let Some(val) = s.strip_prefix("~/") {
            let denom: f64 = val.trim_end_matches('.').parse().unwrap_or(4.0);
            format!("~/{}", (denom / 2.0).max(1.0) as i32)
        } else {
            s.to_string()
        }
    } else if let Some(val) = s.strip_prefix("1/") {
        let dotted = val.ends_with('.');
        let num_str = val.trim_end_matches('.');
        let denom: f64 = num_str.parse().unwrap_or(4.0);
        let new_denom = (denom / 2.0).max(1.0) as i32;
        if dotted {
            format!("1/{new_denom}.")
        } else {
            format!("1/{new_denom}")
        }
    } else {
        s.to_string()
    }
}

/// Pad or truncate rhythm/contour/emphasis to fill exactly one bar.
fn pad_to_bar(
    rhythm: &[String],
    contour: &[String],
    emphasis: &[String],
    bar_beats: f64,
    content_beats: f64,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut r = rhythm.to_vec();
    let mut c = contour.to_vec();
    let mut e = emphasis.to_vec();

    if content_beats < bar_beats - 0.01 {
        // Pad with rest
        let remaining = bar_beats - content_beats;
        let rest = beats_to_rest(remaining);
        r.push(rest);
        c.push("~".into());
        e.push("~".into());
    } else if content_beats > bar_beats + 0.01 {
        // Truncate to fit bar (approximate — remove last elements)
        let parsed = rhythm::parse_rhythm(&r).unwrap_or_else(|_| rhythm::ParsedRhythm {
            hits: vec![],
            offsets: vec![],
            total_beats: 0.0,
        });
        let mut keep = r.len();
        let mut total = parsed.total_beats;
        while keep > 0 && total > bar_beats + 0.01 {
            keep -= 1;
            total = parsed.offsets.get(keep).copied().unwrap_or(0.0);
        }
        r.truncate(keep);
        c.truncate(keep);
        e.truncate(keep);
        // Pad remaining
        let remaining = bar_beats - total;
        if remaining > 0.01 {
            r.push(beats_to_rest(remaining));
            c.push("~".into());
            e.push("~".into());
        }
    }

    // Ensure emphasis matches length
    while e.len() < r.len() {
        e.push("medium".into());
    }
    e.truncate(r.len());

    (r, c, e)
}

/// Convert beats to a rest string.
fn beats_to_rest(beats: f64) -> String {
    // Find the closest standard denomination
    let denom = 4.0 / beats;
    if (denom - denom.round()).abs() < 0.01 {
        let d = denom.round() as i32;
        if d == 1 {
            return "~/1".into(); // whole rest
        }
        format!("~/{d}")
    } else {
        // Approximate as dotted or just use closest
        format!("~/{}", (denom.round().max(1.0)) as i32)
    }
}

/// Get motif emphasis, defaulting to alternating strong/weak if not specified.
fn motif_emphasis(motif: &MotifSpec) -> Vec<String> {
    if !motif.emphasis.is_empty() {
        motif.emphasis.clone()
    } else {
        motif
            .rhythm
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if i == 0 {
                    "strong".into()
                } else {
                    "weak".into()
                }
            })
            .collect()
    }
}

/// Build departure material — contrasting rhythm and wider intervals.
fn build_departure(bar_beats: f64, bias: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
    let (leap_up, leap_down) = match bias {
        "up" => ("leap_up_3", "step_up"),
        "down" => ("step_down", "leap_down_3"),
        _ => ("leap_up_3", "leap_down_2"),
    };

    if bar_beats >= 4.0 {
        (
            vec!["1/4".into(), "1/8".into(), "1/8".into(), "1/4".into(), "1/4".into()],
            vec![
                leap_up.into(),
                "step_down".into(),
                "step_down".into(),
                leap_down.into(),
                "step_up".into(),
            ],
            vec![
                "strong".into(),
                "weak".into(),
                "weak".into(),
                "strong".into(),
                "medium".into(),
            ],
        )
    } else {
        // Shorter bar (3/4, etc.)
        (
            vec!["1/4".into(), "1/8".into(), "1/8".into(), "1/4".into()],
            vec![
                leap_up.into(),
                "step_down".into(),
                leap_down.into(),
                "step_up".into(),
            ],
            vec![
                "strong".into(),
                "weak".into(),
                "strong".into(),
                "medium".into(),
            ],
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_motif() -> MotifSpec {
        MotifSpec {
            rhythm: vec!["1/8".into(), "1/8".into(), "1/4".into()],
            contour: vec!["root".into(), "step_up".into(), "leap_up_2".into()],
            emphasis: vec!["medium".into(), "weak".into(), "strong".into()],
        }
    }

    #[test]
    fn test_invert_contour() {
        let contour = vec![
            "root".into(),
            "step_up".into(),
            "leap_up_3".into(),
            "step_down".into(),
        ];
        let inverted = invert_contour(&contour);
        assert_eq!(inverted, vec!["root", "step_down", "leap_down_3", "step_up"]);
    }

    #[test]
    fn test_apply_degree_offset_zero() {
        let contour = vec!["root".into(), "step_up".into()];
        let result = apply_degree_offset(&contour, 0);
        assert_eq!(result, contour);
    }

    #[test]
    fn test_apply_degree_offset_up() {
        let contour = vec!["root".into(), "step_up".into()];
        let result = apply_degree_offset(&contour, 2);
        assert_eq!(result[0], "leap_up_2");
        assert_eq!(result[1], "step_up"); // relative motion unchanged
    }

    #[test]
    fn test_augment_rhythm() {
        assert_eq!(augment_rhythm("1/8"), "1/4");
        assert_eq!(augment_rhythm("1/4"), "1/2");
        assert_eq!(augment_rhythm("1/16"), "1/8");
    }

    #[test]
    fn test_statement_pads_to_bar() {
        let motif = test_motif();
        // Motif is 1/8 + 1/8 + 1/4 = 0.5 + 0.5 + 1.0 = 2 beats. Bar = 4 beats.
        let (r, c, e) = expand_transform(
            "statement", &motif, 4.0, 2.0,
            &[], &[], &[], &mut 0,
        ).unwrap();
        // Should have motif + rest to fill remaining 3 beats
        assert_eq!(c.last().unwrap(), "~");
        // Total should parse to ~4 beats
        let parsed = rhythm::parse_rhythm(&r).unwrap();
        assert!((parsed.total_beats - 4.0).abs() < 0.1);
    }

    #[test]
    fn test_sequence_up_shifts_root() {
        let motif = test_motif();
        let mut offset = 0;
        let (_, c1, _) = expand_transform(
            "statement", &motif, 4.0, 2.0,
            &[], &[], &[], &mut offset,
        ).unwrap();
        assert_eq!(c1[0], "root");

        let (_, c2, _) = expand_transform(
            "sequence_up", &motif, 4.0, 2.0,
            &[], &[], &[], &mut offset,
        ).unwrap();
        assert_eq!(c2[0], "leap_up_1");
    }

    #[test]
    fn test_resolve_ends_on_root() {
        let motif = test_motif();
        let (_, c, _) = expand_transform(
            "resolve", &motif, 4.0, 2.0,
            &[], &[], &[], &mut 0,
        ).unwrap();
        // Last non-rest contour should be "root"
        let last_note = c.iter().rev().find(|t| t.trim() != "~").unwrap();
        assert_eq!(last_note, "root");
    }

    #[test]
    fn test_expand_simple_motif() {
        let yaml = r#"
name: Test Motif
type: melody
time: "4/4"
motif:
  rhythm: ["1/8", "1/8", "1/4"]
  contour: [root, step_up, leap_up_2]
  emphasis: [medium, weak, strong]
structure:
  - statement
  - repeat
  - sequence_up
  - resolve
"#;
        let pattern = PatternFile::from_yaml(yaml).unwrap();
        let expanded = expand_motif(&pattern, (4, 4)).unwrap();

        // Should have rhythm and contour arrays
        assert!(expanded.rhythm.is_some());
        assert!(expanded.contour.is_some());

        // Should be 4 bars worth of material
        let parsed = rhythm::parse_rhythm(expanded.rhythm_hits()).unwrap();
        assert!(
            (parsed.total_beats - 16.0).abs() < 0.5,
            "Expected ~16 beats, got {}",
            parsed.total_beats
        );
    }

    #[test]
    fn test_expand_with_default_structure() {
        let yaml = r#"
name: Auto Structure
type: melody
complexity: simple
time: "4/4"
motif:
  rhythm: ["1/8", "1/8", "1/4"]
  contour: [root, step_up, leap_up_2]
"#;
        let pattern = PatternFile::from_yaml(yaml).unwrap();
        let expanded = expand_motif(&pattern, (4, 4)).unwrap();

        // Simple = 4 bars = 16 beats
        let parsed = rhythm::parse_rhythm(expanded.rhythm_hits()).unwrap();
        assert!(
            (parsed.total_beats - 16.0).abs() < 0.5,
            "Expected ~16 beats, got {}",
            parsed.total_beats
        );
    }
}
