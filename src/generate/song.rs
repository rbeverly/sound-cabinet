//! Song expander: assembles multi-part compositions from named parts
//! (verse, refrain, bridge, etc.) with an arrangement order.
//!
//! Each part has its own motif, complexity, and optional chord/range overrides.
//! Parts are expanded independently through the motif → resolver → variation
//! pipeline, then assembled into a single .sc file with sections.

use std::collections::HashMap;

use anyhow::{anyhow, Result};

use super::motif;
use super::pattern::{PatternFile, SongFile};
use super::resolver::{self, GenerateParams, ResolvedNote};
use super::rhythm;
use super::theory::{Chord, Mode, PitchClass, PitchRange, Scale};
use super::variation;
use super::writer;
use super::GenerateConfig;

/// Resolved output for one part of a song.
struct ResolvedPart {
    /// Name of the part (e.g. "verse", "refrain").
    name: String,
    /// All variations of this part.
    variations: Vec<Vec<ResolvedNote>>,
    /// Duration in beats of one instance.
    total_beats: f64,
}

/// Run the full song generation pipeline.
pub fn run_generate_song(song: &SongFile, config: &GenerateConfig) -> Result<()> {
    let root = PitchClass::parse(&config.key)?;
    let mode = Mode::parse(&config.mode)?;
    let scale = Scale::new(root, mode);
    let time_sig = rhythm::parse_time_sig(&song.time)?;

    // Default chords from CLI
    let default_chords: Vec<Chord> = config
        .chords
        .split_whitespace()
        .map(|c| Chord::parse(c))
        .collect::<Result<Vec<_>>>()?;

    if default_chords.is_empty() {
        return Err(anyhow!("At least one chord is required"));
    }

    let default_range = super::default_range("melody");

    // Expand each unique part
    let mut resolved_parts: HashMap<String, ResolvedPart> = HashMap::new();

    for part_name in &song.arrangement {
        if resolved_parts.contains_key(part_name) {
            continue; // Already resolved
        }

        let part = song
            .parts
            .get(part_name)
            .ok_or_else(|| anyhow!("Part '{}' not found", part_name))?;

        // Build a temporary PatternFile from the part's motif
        let temp_pattern = PatternFile {
            name: format!("{} - {}", song.name, part_name),
            pattern_type: "melody".into(),
            tags: vec![],
            time: song.time.clone(),
            rhythm: None,
            contour: None,
            emphasis: vec![],
            motif: Some(part.motif.clone()),
            structure: part.structure.clone(),
            complexity: part.complexity.clone(),
            notes: None,
        };

        // Expand the motif into a full pattern
        let expanded = motif::expand_motif(&temp_pattern, time_sig)?;

        // Use part-specific chords if provided, else global
        let chords = if let Some(ref chord_str) = part.chords {
            chord_str
                .split_whitespace()
                .map(|c| Chord::parse(c))
                .collect::<Result<Vec<_>>>()?
        } else {
            default_chords.clone()
        };

        // Use part-specific range if provided
        let range = if let Some(ref r) = part.range {
            PitchRange::parse(r)?
        } else if let Some(ref r) = config.range {
            PitchRange::parse(r)?
        } else {
            default_range
        };

        let params = GenerateParams {
            scale,
            chords,
            range,
            voice_name: config.voice.clone(),
            time_sig,
        };

        // Get total beats
        let parsed_rhythm = rhythm::parse_rhythm(expanded.rhythm_hits())?;
        let total_beats = parsed_rhythm.total_beats;

        // Generate variations
        let variations = variation::generate_variations(
            &expanded,
            &params,
            config.variations,
            config.seed,
        )?;

        resolved_parts.insert(
            part_name.clone(),
            ResolvedPart {
                name: part_name.clone(),
                variations,
                total_beats,
            },
        );
    }

    // Write output
    let output = write_song_output(
        &song.name,
        &config.key,
        &config.mode,
        &config.chords,
        root,
        &config.voice,
        &song.arrangement,
        &resolved_parts,
        config.variations,
    );

    if let Some(ref path) = config.output {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, &output)?;
        eprintln!(
            "Generated song with {} parts, {} variations -> {}",
            resolved_parts.len(),
            config.variations,
            path
        );
    } else {
        print!("{output}");
    }

    Ok(())
}

/// Write the full song as .sc output.
fn write_song_output(
    song_name: &str,
    key_name: &str,
    mode_name: &str,
    chord_names: &str,
    key_root: PitchClass,
    voice_name: &str,
    arrangement: &[String],
    parts: &HashMap<String, ResolvedPart>,
    variation_count: usize,
) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "// Generated song: {}\n// Key: {} {}, Chords: {}\n\n",
        song_name, key_name, mode_name, chord_names
    ));

    // Write each part's patterns (all variations)
    for (part_name, part) in parts {
        out.push_str(&format!(
            "// --- {} ({} beats) ---\n",
            part_name, writer::format_beats_pub(part.total_beats)
        ));

        for (vi, notes) in part.variations.iter().enumerate() {
            let letter = writer::variation_letter_pub(vi);
            let pattern_name = format!("{}_{}", part_name, letter);

            out.push_str(&format!(
                "pattern {} = {} beats\n",
                pattern_name,
                writer::format_beats_pub(part.total_beats)
            ));

            for note in notes {
                if note.is_rest {
                    continue;
                }
                let note_name = note.pitch.to_note_name(Some(key_root));
                let beat_str = writer::format_beats_pub(note.beat_offset);
                let dur_str = writer::format_beats_pub(note.duration_beats);

                if (note.velocity - 1.0).abs() < 0.01 {
                    out.push_str(&format!(
                        "  at {beat_str} play {voice_name}({note_name}) for {dur_str} beats\n"
                    ));
                } else {
                    out.push_str(&format!(
                        "  at {beat_str} play {voice_name}({note_name}) * {vel} for {dur_str} beats\n",
                        vel = writer::format_velocity_pub(note.velocity)
                    ));
                }
            }
            out.push('\n');
        }
    }

    // Write arrangement as comments (for reference when building a score)
    out.push_str("// --- Arrangement ---\n");
    out.push_str(&format!(
        "// {}\n",
        arrangement.join(", ")
    ));
    out.push_str("//\n// Example usage:\n");

    let letter = writer::variation_letter_pub(0);
    for part_name in arrangement {
        let pattern_name = format!("{}_{}", part_name, letter);
        out.push_str(&format!("// play {}\n", pattern_name));
    }
    out.push('\n');

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::pattern::SongFile;

    #[test]
    fn test_song_file_parse() {
        let yaml = r#"
name: Test Song
time: "4/4"
parts:
  verse:
    motif:
      rhythm: ["1/8", "1/8", "1/4"]
      contour: [root, step_up, leap_up_2]
    complexity: simple
  refrain:
    motif:
      rhythm: ["1/4", "1/4", "1/2"]
      contour: [leap_up_4, step_down, root]
      emphasis: [strong, strong, strong]
    complexity: simple
arrangement: [verse, verse, refrain, verse, refrain]
"#;
        let song = SongFile::from_yaml(yaml).unwrap();
        assert_eq!(song.parts.len(), 2);
        assert_eq!(song.arrangement.len(), 5);
    }

    #[test]
    fn test_song_invalid_arrangement() {
        let yaml = r#"
name: Bad Song
time: "4/4"
parts:
  verse:
    motif:
      rhythm: ["1/4"]
      contour: [root]
arrangement: [verse, chorus]
"#;
        assert!(SongFile::from_yaml(yaml).is_err());
    }

    #[test]
    fn test_generate_song() {
        let yaml = r#"
name: Test Song
time: "4/4"
parts:
  verse:
    motif:
      rhythm: ["1/8", "1/8", "1/4"]
      contour: [root, step_up, leap_up_2]
    complexity: simple
  refrain:
    motif:
      rhythm: ["1/4", "1/4", "1/2"]
      contour: [leap_up_4, step_down, root]
    complexity: simple
arrangement: [verse, refrain, verse, refrain]
"#;
        let song = SongFile::from_yaml(yaml).unwrap();
        let config = GenerateConfig {
            pattern_path: String::new(),
            key: "C".into(),
            mode: "major".into(),
            chords: "Cmaj Fmaj Gmaj Cmaj".into(),
            voice: "mel".into(),
            range: None,
            variations: 2,
            seed: 42,
            output: None,
        };

        // Should not panic
        let result = run_generate_song(&song, &config);
        assert!(result.is_ok(), "Song generation failed: {:?}", result.err());
    }
}
