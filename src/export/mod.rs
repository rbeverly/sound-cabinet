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

    // Reject non-finite note timings before rendering. The DSL `number`
    // grammar accepts arbitrarily long digit strings, which `parse::<f64>()`
    // silently maps to `f64::INFINITY` on overflow. A non-finite beat or
    // duration would otherwise drive the LilyPond rest-fill loop into an
    // unbounded spin (a denial of service). Fail with a clear error instead.
    check_finite_timings(&notes)?;

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

/// Validate that every note's beat position and duration is a finite number.
///
/// Returns an error identifying the first offending value if any note's `beat`
/// or `duration_beats` is infinite or NaN. This prevents non-finite timings —
/// produced when an overflowing digit string parses to `f64::INFINITY` — from
/// reaching the LilyPond rest-fill routine, where they would cause an
/// unbounded loop.
fn check_finite_timings(notes: &[extract::NoteEvent]) -> Result<()> {
    for n in notes {
        if !n.beat.is_finite() || !n.duration_beats.is_finite() {
            return Err(anyhow!(
                "Cannot export note with non-finite timing: beat {}, duration {}",
                n.beat,
                n.duration_beats
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::ast::{Command, Expr};

    fn note_event(beat: f64, duration_beats: f64) -> extract::NoteEvent {
        extract::NoteEvent {
            beat,
            pitch: None,
            duration_beats,
            voice_name: "piano".to_string(),
            voice_label: None,
            velocity: 1.0,
            source: None,
        }
    }

    #[test]
    fn export_rejects_nonfinite_duration() {
        // Mirror the real pipeline: an overflowing duration parses to infinity,
        // flows through extraction, and must be rejected before rendering.
        let commands = vec![Command::PlayAt {
            beat: 0.0,
            expr: Expr::FnCall {
                name: "piano".to_string(),
                args: vec![Expr::Number(440.0)],
            },
            duration_beats: f64::INFINITY,
            source: None,
            voice_label: None,
            velocity: 1.0,
        }];
        let extracted = extract::extract_notes(&commands);
        let result = check_finite_timings(&extracted.notes);
        assert!(result.is_err(), "non-finite duration should be rejected");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("non-finite timing"),
            "error should identify the non-finite timing, got: {msg}"
        );
    }

    #[test]
    fn export_rejects_nonfinite_beat() {
        let notes = vec![note_event(f64::INFINITY, 1.0)];
        assert!(check_finite_timings(&notes).is_err());
    }

    #[test]
    fn export_rejects_nan_timing() {
        let notes = vec![note_event(0.0, f64::NAN)];
        assert!(check_finite_timings(&notes).is_err());
    }

    #[test]
    fn export_accepts_finite_timings() {
        // A normal finite score passes the finiteness check.
        let notes = vec![note_event(0.0, 1.0), note_event(2.0, 0.5)];
        assert!(check_finite_timings(&notes).is_ok());
    }
}
