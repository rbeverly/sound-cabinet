//! YAML pattern file loading and validation.

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// A YAML pattern file describing a reusable musical gesture.
/// Can be either a direct pattern (rhythm + contour) or a motif-based pattern
/// (motif + structure, which gets expanded into rhythm + contour).
#[derive(Debug, Deserialize, Clone)]
pub struct PatternFile {
    pub name: String,

    #[serde(rename = "type")]
    pub pattern_type: String,

    #[serde(default)]
    pub tags: Vec<String>,

    pub time: String,

    /// Direct pattern: explicit rhythm array. Optional if `motif` is provided.
    #[serde(default)]
    pub rhythm: Option<RhythmSpec>,

    /// Direct pattern: explicit contour array. Optional if `motif` is provided.
    #[serde(default)]
    pub contour: Option<Vec<String>>,

    #[serde(default)]
    pub emphasis: Vec<String>,

    /// Motif-based pattern: a short musical idea to be transformed.
    #[serde(default)]
    pub motif: Option<MotifSpec>,

    /// Motif-based pattern: sequence of transformations to apply.
    #[serde(default)]
    pub structure: Option<Vec<String>>,

    /// Motif-based pattern: complexity level (simple, moderate, complex).
    /// Used to auto-generate structure when structure is omitted.
    #[serde(default)]
    pub complexity: Option<String>,

    /// Human-readable description (not used by generator).
    #[serde(default)]
    pub notes: Option<String>,
}

/// A short musical motif to be expanded via transformations.
#[derive(Debug, Deserialize, Clone)]
pub struct MotifSpec {
    pub rhythm: Vec<String>,
    pub contour: Vec<String>,
    #[serde(default)]
    pub emphasis: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RhythmSpec {
    pub hits: Vec<String>,
}

impl PatternFile {
    /// Load and validate a pattern file from disk.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Cannot read pattern file {}: {}", path.display(), e))?;
        let pattern: PatternFile = serde_yaml::from_str(&contents)
            .map_err(|e| anyhow!("Invalid YAML in {}: {}", path.display(), e))?;
        pattern.validate()?;
        Ok(pattern)
    }

    /// Parse from a YAML string (for testing).
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let pattern: PatternFile = serde_yaml::from_str(yaml)
            .map_err(|e| anyhow!("Invalid YAML: {e}"))?;
        pattern.validate()?;
        Ok(pattern)
    }

    /// Get rhythm hits (panics if not a direct pattern — call after expansion).
    pub fn rhythm_hits(&self) -> &[String] {
        &self.rhythm.as_ref().expect("rhythm not set — expand motif first").hits
    }

    /// Get contour tokens (panics if not a direct pattern — call after expansion).
    pub fn contour_tokens(&self) -> &[String] {
        self.contour.as_ref().expect("contour not set — expand motif first")
    }

    /// Returns true if this is a motif-based pattern that needs expansion.
    pub fn has_motif(&self) -> bool {
        self.motif.is_some()
    }

    /// Validate internal consistency.
    fn validate(&self) -> Result<()> {
        // Must have either (rhythm + contour) or motif
        let has_direct = self.rhythm.is_some() && self.contour.is_some();
        let has_motif = self.motif.is_some();

        if !has_direct && !has_motif {
            return Err(anyhow!(
                "Pattern '{}': must have either rhythm+contour or motif",
                self.name
            ));
        }

        // Validate direct pattern fields if present
        if let (Some(rhythm), Some(contour)) = (&self.rhythm, &self.contour) {
            if rhythm.hits.len() != contour.len() {
                return Err(anyhow!(
                    "Pattern '{}': rhythm has {} hits but contour has {} entries",
                    self.name,
                    rhythm.hits.len(),
                    contour.len()
                ));
            }

            if !self.emphasis.is_empty() && self.emphasis.len() != rhythm.hits.len() {
                return Err(anyhow!(
                    "Pattern '{}': emphasis has {} entries but rhythm has {} hits",
                    self.name,
                    self.emphasis.len(),
                    rhythm.hits.len()
                ));
            }

            // Rest alignment
            for (i, (hit, cont)) in rhythm.hits.iter().zip(contour.iter()).enumerate() {
                let hit_is_rest = hit.trim() == "~" || hit.trim().starts_with("~/");
                let contour_is_rest = cont.trim() == "~";

                if hit_is_rest && !contour_is_rest {
                    return Err(anyhow!(
                        "Pattern '{}': rhythm position {} is a rest but contour is '{}' (expected '~')",
                        self.name, i + 1, cont
                    ));
                }
                if !hit_is_rest && contour_is_rest {
                    return Err(anyhow!(
                        "Pattern '{}': contour position {} is '~' but rhythm is '{}' (expected a rest)",
                        self.name, i + 1, hit
                    ));
                }
            }
        }

        // Validate motif if present
        if let Some(motif) = &self.motif {
            if motif.rhythm.len() != motif.contour.len() {
                return Err(anyhow!(
                    "Pattern '{}': motif rhythm has {} entries but motif contour has {}",
                    self.name,
                    motif.rhythm.len(),
                    motif.contour.len()
                ));
            }
            if !motif.emphasis.is_empty() && motif.emphasis.len() != motif.rhythm.len() {
                return Err(anyhow!(
                    "Pattern '{}': motif emphasis has {} entries but motif rhythm has {}",
                    self.name,
                    motif.emphasis.len(),
                    motif.rhythm.len()
                ));
            }
        }

        // Validate time signature format
        super::rhythm::parse_time_sig(&self.time)?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Song files (multi-part compositions)
// ---------------------------------------------------------------------------

/// A song file: multiple named parts (verse, refrain, bridge, etc.)
/// with an arrangement specifying their order.
#[derive(Debug, Deserialize)]
pub struct SongFile {
    pub name: String,
    pub time: String,
    pub parts: HashMap<String, SongPart>,
    pub arrangement: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// A single part of a song (verse, refrain, bridge, etc.).
#[derive(Debug, Deserialize)]
pub struct SongPart {
    pub motif: MotifSpec,
    #[serde(default)]
    pub complexity: Option<String>,
    #[serde(default)]
    pub structure: Option<Vec<String>>,
    /// Per-part chord override (space-separated, e.g. "Am Dm Em Am").
    #[serde(default)]
    pub chords: Option<String>,
    /// Per-part range override (e.g. "C4-C6").
    #[serde(default)]
    pub range: Option<String>,
}

impl SongFile {
    /// Load and validate a song file from disk.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Cannot read song file {}: {}", path.display(), e))?;
        Self::from_yaml(&contents)
    }

    /// Parse from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let song: SongFile = serde_yaml::from_str(yaml)
            .map_err(|e| anyhow!("Invalid song YAML: {e}"))?;
        song.validate()?;
        Ok(song)
    }

    fn validate(&self) -> Result<()> {
        if self.parts.is_empty() {
            return Err(anyhow!("Song '{}': must have at least one part", self.name));
        }
        if self.arrangement.is_empty() {
            return Err(anyhow!(
                "Song '{}': arrangement must have at least one entry",
                self.name
            ));
        }
        // Verify all arrangement entries reference defined parts
        for entry in &self.arrangement {
            if !self.parts.contains_key(entry) {
                return Err(anyhow!(
                    "Song '{}': arrangement references undefined part '{}'",
                    self.name,
                    entry
                ));
            }
        }
        // Validate motifs within each part
        for (name, part) in &self.parts {
            if part.motif.rhythm.len() != part.motif.contour.len() {
                return Err(anyhow!(
                    "Song '{}', part '{}': motif rhythm has {} entries but contour has {}",
                    self.name,
                    name,
                    part.motif.rhythm.len(),
                    part.motif.contour.len()
                ));
            }
        }
        super::rhythm::parse_time_sig(&self.time)?;
        Ok(())
    }
}

/// Map emphasis string to velocity (0.0 to 1.0).
pub fn emphasis_to_velocity(s: &str) -> f64 {
    match s.trim().to_lowercase().as_str() {
        "strong" => 1.0,
        "medium" | "med" => 0.7,
        "weak" => 0.4,
        "ghost" => 0.2,
        _ => 0.7, // default to medium
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_walking_jazz() {
        let yaml = r#"
name: Walking Jazz Bass
type: bass
tags: [jazz, walking, quarter-note]
time: "4/4"
rhythm:
  hits: ["1/4", "1/4", "1/4", "1/4"]
contour: [root, step_up, step_up, approach]
emphasis: [strong, weak, weak, medium]
"#;
        let pattern = PatternFile::from_yaml(yaml).unwrap();
        assert_eq!(pattern.name, "Walking Jazz Bass");
        assert_eq!(pattern.pattern_type, "bass");
        assert_eq!(pattern.rhythm_hits().len(), 4);
        assert_eq!(pattern.contour_tokens().len(), 4);
        assert_eq!(pattern.emphasis.len(), 4);
    }

    #[test]
    fn test_load_with_rests() {
        let yaml = r#"
name: Root Fifth
type: bass
time: "4/4"
rhythm:
  hits: ["1/4", "~/4", "1/4", "~/4"]
contour: [root, "~", leap_up_4, "~"]
emphasis: [strong, "~", medium, "~"]
"#;
        let pattern = PatternFile::from_yaml(yaml).unwrap();
        assert_eq!(pattern.rhythm_hits().len(), 4);
    }

    #[test]
    fn test_mismatched_lengths_rejected() {
        let yaml = r#"
name: Bad
type: bass
time: "4/4"
rhythm:
  hits: ["1/4", "1/4", "1/4"]
contour: [root, step_up]
"#;
        assert!(PatternFile::from_yaml(yaml).is_err());
    }

    #[test]
    fn test_rest_mismatch_rejected() {
        let yaml = r#"
name: Bad
type: bass
time: "4/4"
rhythm:
  hits: ["~/4", "1/4"]
contour: [root, step_up]
"#;
        assert!(PatternFile::from_yaml(yaml).is_err());
    }

    #[test]
    fn test_emphasis_to_velocity() {
        assert!((emphasis_to_velocity("strong") - 1.0).abs() < 1e-10);
        assert!((emphasis_to_velocity("weak") - 0.4).abs() < 1e-10);
        assert!((emphasis_to_velocity("ghost") - 0.2).abs() < 1e-10);
        assert!((emphasis_to_velocity("medium") - 0.7).abs() < 1e-10);
    }
}
