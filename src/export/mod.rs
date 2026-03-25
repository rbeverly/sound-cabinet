//! Sheet music export via LilyPond.
//!
//! Parses a .sc file, extracts note events, and outputs LilyPond (.ly) format
//! for rendering to PDF sheet music.

pub mod extract;
pub mod lilypond;

use anyhow::{anyhow, Result};
use std::path::Path;

use crate::dsl::{expand::expand_script, import::resolve_imports, parser::parse_script};

/// Configuration for an export run.
pub struct ExportConfig {
    pub score_path: String,
    pub output: String,
    pub format: ExportFormat,
    pub voice_filter: Option<String>,
    pub source_filter: Option<String>,
    pub from_beat: Option<f64>,
    pub to_beat: Option<f64>,
    pub time_sig: String,
    pub key: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportFormat {
    Lilypond,
    Pdf,
}

/// Run the export pipeline.
pub fn run_export(config: &ExportConfig) -> Result<()> {
    // Parse and expand the score
    let source = std::fs::read_to_string(&config.score_path)
        .map_err(|e| anyhow!("Cannot read {}: {e}", config.score_path))?;
    let script = parse_script(&source)?;
    let base_dir = Path::new(&config.score_path)
        .parent()
        .unwrap_or(Path::new("."));
    let script = resolve_imports(script, base_dir)?;
    let script = expand_script(script, &mut rand::thread_rng())?;

    // Extract note events
    let extracted = extract::extract_notes(&script.commands);

    // Apply filters
    let mut notes = extracted.notes;

    if let Some(ref voice) = config.voice_filter {
        notes.retain(|n| {
            // Match against voice_label first (preserves names across `with` substitution),
            // then fall back to the resolved voice_name.
            n.voice_label.as_deref() == Some(voice.as_str()) || n.voice_name == *voice
        });
    }
    if let Some(ref source) = config.source_filter {
        notes.retain(|n| {
            n.source
                .as_ref()
                .map(|s| s.contains(source.as_str()))
                .unwrap_or(false)
        });
    }
    if let Some(from) = config.from_beat {
        notes.retain(|n| n.beat >= from);
    }
    if let Some(to) = config.to_beat {
        notes.retain(|n| n.beat < to);
    }

    // Shift beats if --from was specified (so the output starts at beat 0)
    if let Some(from) = config.from_beat {
        for note in &mut notes {
            note.beat -= from;
        }
    }

    if notes.is_empty() {
        return Err(anyhow!("No notes to export (check voice/source filters)"));
    }

    // Generate LilyPond
    let ly_output = lilypond::write_lilypond(&notes, &extracted.tempos, config);

    match config.format {
        ExportFormat::Lilypond => {
            std::fs::write(&config.output, &ly_output)?;
            eprintln!(
                "Exported {} notes across {} voices -> {}",
                notes.len(),
                count_voices(&notes),
                config.output
            );
        }
        ExportFormat::Pdf => {
            // Write .ly to a temp file, then call lilypond
            let ly_path = format!("{}.ly", config.output.trim_end_matches(".pdf"));
            std::fs::write(&ly_path, &ly_output)?;

            let pdf_base = config.output.trim_end_matches(".pdf");
            let result = std::process::Command::new("lilypond")
                .args(["-o", pdf_base, &ly_path])
                .output();

            match result {
                Ok(output) => {
                    if output.status.success() {
                        eprintln!(
                            "Exported {} notes -> {} (via LilyPond)",
                            notes.len(),
                            config.output
                        );
                        // Clean up temp .ly file
                        let _ = std::fs::remove_file(&ly_path);
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprintln!("LilyPond errors:\n{stderr}");
                        eprintln!("LilyPond source saved to: {ly_path}");
                        return Err(anyhow!("LilyPond failed to render PDF"));
                    }
                }
                Err(_) => {
                    eprintln!("LilyPond source saved to: {ly_path}");
                    eprintln!();
                    eprintln!("To render PDF, install LilyPond:");
                    eprintln!("  macOS:   brew install lilypond");
                    eprintln!("  Ubuntu:  sudo apt install lilypond");
                    eprintln!("  Windows: https://lilypond.org/download.html");
                    eprintln!();
                    eprintln!("Then run: lilypond -o {} {}", pdf_base, ly_path);
                    return Err(anyhow!(
                        "LilyPond not found — install it to render PDF"
                    ));
                }
            }
        }
    }

    Ok(())
}

fn count_voices(notes: &[extract::NoteEvent]) -> usize {
    let mut voices: Vec<&str> = notes.iter().map(|n| n.voice_name.as_str()).collect();
    voices.sort();
    voices.dedup();
    voices.len()
}
