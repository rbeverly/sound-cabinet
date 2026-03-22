//! Algorithmic phrase generation.
//!
//! Reads YAML pattern files and resolves them against musical parameters
//! (key, mode, chord progression, instrument range) to produce .sc output
//! with named pattern variations.

pub mod drums;
pub mod motif;
pub mod pattern;
pub mod resolver;
pub mod rhythm;
pub mod song;
pub mod theory;
pub mod variation;
pub mod writer;

use anyhow::{anyhow, Result};
use std::path::Path;

use pattern::PatternFile;
use resolver::GenerateParams;
use theory::{Chord, Mode, PitchClass, PitchRange, Scale};

/// Configuration for a generate run, parsed from CLI args.
pub struct GenerateConfig {
    pub pattern_path: String,
    pub key: String,
    pub mode: String,
    pub chords: String,
    pub voice: String,
    pub range: Option<String>,
    pub variations: usize,
    pub seed: u64,
    pub output: Option<String>,
}

/// Run the full generation pipeline.
pub fn run_generate(config: &GenerateConfig) -> Result<()> {
    // Try song file first (has "parts" key)
    if let Ok(song_file) = pattern::SongFile::load(Path::new(&config.pattern_path)) {
        return song::run_generate_song(&song_file, config);
    }

    // Try drum pattern (has "voices" key)
    if let Ok(drum_pat) = pattern::DrumPattern::load(Path::new(&config.pattern_path)) {
        return run_generate_drums(&drum_pat, config);
    }

    // Load as pattern/motif file
    let pattern = PatternFile::load(Path::new(&config.pattern_path))?;

    // Parse musical parameters
    let root = PitchClass::parse(&config.key)?;
    let mode = Mode::parse(&config.mode)?;
    let scale = Scale::new(root, mode);

    let chords: Vec<Chord> = config
        .chords
        .split_whitespace()
        .map(|c| Chord::parse(c))
        .collect::<Result<Vec<_>>>()?;

    if chords.is_empty() {
        return Err(anyhow!("At least one chord is required"));
    }

    let time_sig = rhythm::parse_time_sig(&pattern.time)?;

    // If this is a motif-based pattern, expand it first
    let pattern = if pattern.has_motif() {
        motif::expand_motif(&pattern, time_sig)?
    } else {
        pattern
    };

    let range = if let Some(ref r) = config.range {
        PitchRange::parse(r)?
    } else {
        default_range(&pattern.pattern_type)
    };

    let params = GenerateParams {
        scale,
        chords,
        range,
        voice_name: config.voice.clone(),
        time_sig,
    };

    // Parse rhythm to get total beats
    let parsed_rhythm = rhythm::parse_rhythm(pattern.rhythm_hits())?;

    // Generate variations
    let variations = variation::generate_variations(
        &pattern,
        &params,
        config.variations,
        config.seed,
    )?;

    // Write output
    let output = writer::write_sc(
        &variations,
        &config.voice,
        &pattern.name,
        &config.key,
        &config.mode,
        &config.chords,
        root,
        parsed_rhythm.total_beats,
    );

    if let Some(ref path) = config.output {
        // Ensure parent directory exists
        if let Some(parent) = Path::new(path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, &output)?;
        eprintln!(
            "Generated {} variations -> {}",
            config.variations, path
        );
    } else {
        // Print to stdout
        print!("{output}");
    }

    Ok(())
}

/// Run drum pattern generation.
fn run_generate_drums(drum_pat: &pattern::DrumPattern, config: &GenerateConfig) -> Result<()> {
    let time_sig = rhythm::parse_time_sig(&drum_pat.time)?;

    // Get total beats from the longest voice
    let mut total_beats = 0.0_f64;
    for dv in &drum_pat.voices {
        let parsed = rhythm::parse_rhythm(&dv.rhythm)?;
        total_beats = total_beats.max(parsed.total_beats);
    }

    let variations = drums::generate_drum_variations(drum_pat, config.variations, config.seed)?;
    let output = drums::write_drum_sc(&variations, &drum_pat.name, total_beats);

    if let Some(ref path) = config.output {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, &output)?;
        eprintln!(
            "Generated {} drum variations -> {}",
            config.variations, path
        );
    } else {
        print!("{output}");
    }

    Ok(())
}

/// Default pitch ranges by instrument type.
fn default_range(pattern_type: &str) -> PitchRange {
    match pattern_type {
        "bass" => PitchRange::new(
            theory::Pitch::from_note_name("C2").unwrap(),
            theory::Pitch::from_note_name("G3").unwrap(),
        ),
        "melody" => PitchRange::new(
            theory::Pitch::from_note_name("C4").unwrap(),
            theory::Pitch::from_note_name("C6").unwrap(),
        ),
        "accomp" | "accompaniment" => PitchRange::new(
            theory::Pitch::from_note_name("C3").unwrap(),
            theory::Pitch::from_note_name("C5").unwrap(),
        ),
        _ => PitchRange::new(
            theory::Pitch::from_note_name("C3").unwrap(),
            theory::Pitch::from_note_name("C5").unwrap(),
        ),
    }
}
