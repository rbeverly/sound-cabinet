//! Rhythm notation parser for YAML pattern files.
//!
//! Parses strings like "1/4", "1/8.", "~/16", "1/4+1/8" into beat durations.

use anyhow::{anyhow, Result};

/// A single rhythmic event: either a sounding note or a rest.
#[derive(Debug, Clone, PartialEq)]
pub enum RhythmHit {
    /// A note with its duration in beats.
    Note(f64),
    /// A rest (silence) with its duration in beats.
    Rest(f64),
}

impl RhythmHit {
    /// Duration in beats regardless of note/rest.
    pub fn duration(&self) -> f64 {
        match self {
            RhythmHit::Note(d) | RhythmHit::Rest(d) => *d,
        }
    }

    pub fn is_rest(&self) -> bool {
        matches!(self, RhythmHit::Rest(_))
    }
}

/// Parsed rhythm: a sequence of hits with their computed beat offsets.
#[derive(Debug, Clone)]
pub struct ParsedRhythm {
    pub hits: Vec<RhythmHit>,
    /// Cumulative beat offset for each hit.
    pub offsets: Vec<f64>,
    /// Total duration in beats.
    pub total_beats: f64,
}

/// Parse a rhythm hits array from YAML into structured rhythm data.
pub fn parse_rhythm(hit_strings: &[String]) -> Result<ParsedRhythm> {
    let mut hits = Vec::with_capacity(hit_strings.len());
    let mut offsets = Vec::with_capacity(hit_strings.len());
    let mut cursor = 0.0;

    for (i, s) in hit_strings.iter().enumerate() {
        let s = s.trim();
        let hit = parse_one_hit(s)
            .map_err(|e| anyhow!("Rhythm hit {} '{}': {}", i + 1, s, e))?;
        offsets.push(cursor);
        cursor += hit.duration();
        hits.push(hit);
    }

    Ok(ParsedRhythm {
        hits,
        offsets,
        total_beats: cursor,
    })
}

/// Parse a single rhythm token.
///
/// Formats:
///   "1/4"      -> Note(1.0)          quarter note
///   "1/8"      -> Note(0.5)          eighth note
///   "1/8."     -> Note(0.75)         dotted eighth
///   "1/4+1/8"  -> Note(1.5)          tied quarter+eighth
///   "~"        -> Rest(1.0)          quarter rest (default)
///   "~/4"      -> Rest(1.0)          quarter rest
///   "~/8"      -> Rest(0.5)          eighth rest
///   "~/8."     -> Rest(0.75)         dotted eighth rest
fn parse_one_hit(s: &str) -> Result<RhythmHit> {
    if s == "~" {
        return Ok(RhythmHit::Rest(1.0)); // default quarter rest
    }

    if let Some(rest_val) = s.strip_prefix("~/") {
        let beats = parse_duration_value(rest_val)?;
        return Ok(RhythmHit::Rest(beats));
    }

    // Tied notes: "1/4+1/8"
    if s.contains('+') {
        let mut total = 0.0;
        for part in s.split('+') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("1/") {
                total += parse_duration_value(val)?;
            } else {
                return Err(anyhow!("Invalid tied duration component: '{part}'"));
            }
        }
        return Ok(RhythmHit::Note(total));
    }

    // Regular note: "1/4", "1/8.", etc.
    if let Some(val) = s.strip_prefix("1/") {
        let beats = parse_duration_value(val)?;
        return Ok(RhythmHit::Note(beats));
    }

    Err(anyhow!("Unrecognized rhythm token: '{s}'"))
}

/// Parse the denominator part of a duration, handling dots.
/// "4" -> 1.0, "8" -> 0.5, "8." -> 0.75, "16" -> 0.25, "2" -> 2.0, "1" -> 4.0
fn parse_duration_value(s: &str) -> Result<f64> {
    let (num_str, dotted) = if s.ends_with('.') {
        (&s[..s.len() - 1], true)
    } else {
        (s, false)
    };

    let denom: f64 = num_str
        .parse()
        .map_err(|_| anyhow!("Invalid duration denominator: '{num_str}'"))?;

    if denom <= 0.0 {
        return Err(anyhow!("Duration denominator must be positive"));
    }

    // In 4/4 time, a whole note = 4 beats, so:
    // 1/1 = 4 beats, 1/2 = 2 beats, 1/4 = 1 beat, 1/8 = 0.5, 1/16 = 0.25
    let beats = 4.0 / denom;

    if dotted {
        Ok(beats * 1.5)
    } else {
        Ok(beats)
    }
}

/// Parse a time signature string like "4/4" or "3/4".
/// Returns (beats_per_bar, beat_unit).
pub fn parse_time_sig(s: &str) -> Result<(u32, u32)> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid time signature: '{s}' (expected N/N)"));
    }
    let num: u32 = parts[0]
        .parse()
        .map_err(|_| anyhow!("Invalid time signature numerator: '{}'", parts[0]))?;
    let denom: u32 = parts[1]
        .parse()
        .map_err(|_| anyhow!("Invalid time signature denominator: '{}'", parts[1]))?;
    Ok((num, denom))
}

/// Convert a time signature to beats per bar.
/// 4/4 = 4 beats, 3/4 = 3 beats, 6/8 = 3 beats (compound time).
pub fn beats_per_bar(numerator: u32, denominator: u32) -> f64 {
    // Each beat unit is (4 / denominator) beats in our system where quarter = 1
    (numerator as f64) * (4.0 / denominator as f64)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quarter_note() {
        assert_eq!(parse_one_hit("1/4").unwrap(), RhythmHit::Note(1.0));
    }

    #[test]
    fn test_eighth_note() {
        assert_eq!(parse_one_hit("1/8").unwrap(), RhythmHit::Note(0.5));
    }

    #[test]
    fn test_sixteenth_note() {
        assert_eq!(parse_one_hit("1/16").unwrap(), RhythmHit::Note(0.25));
    }

    #[test]
    fn test_half_note() {
        assert_eq!(parse_one_hit("1/2").unwrap(), RhythmHit::Note(2.0));
    }

    #[test]
    fn test_whole_note() {
        assert_eq!(parse_one_hit("1/1").unwrap(), RhythmHit::Note(4.0));
    }

    #[test]
    fn test_dotted_eighth() {
        assert_eq!(parse_one_hit("1/8.").unwrap(), RhythmHit::Note(0.75));
    }

    #[test]
    fn test_dotted_quarter() {
        assert_eq!(parse_one_hit("1/4.").unwrap(), RhythmHit::Note(1.5));
    }

    #[test]
    fn test_tied() {
        assert_eq!(parse_one_hit("1/4+1/8").unwrap(), RhythmHit::Note(1.5));
    }

    #[test]
    fn test_bare_rest() {
        assert_eq!(parse_one_hit("~").unwrap(), RhythmHit::Rest(1.0));
    }

    #[test]
    fn test_eighth_rest() {
        assert_eq!(parse_one_hit("~/8").unwrap(), RhythmHit::Rest(0.5));
    }

    #[test]
    fn test_sixteenth_rest() {
        assert_eq!(parse_one_hit("~/16").unwrap(), RhythmHit::Rest(0.25));
    }

    #[test]
    fn test_dotted_eighth_rest() {
        assert_eq!(parse_one_hit("~/8.").unwrap(), RhythmHit::Rest(0.75));
    }

    #[test]
    fn test_half_rest() {
        assert_eq!(parse_one_hit("~/2").unwrap(), RhythmHit::Rest(2.0));
    }

    #[test]
    fn test_parse_rhythm_offsets() {
        let hits: Vec<String> = vec!["1/4", "1/8", "~/8", "1/4"]
            .into_iter()
            .map(String::from)
            .collect();
        let parsed = parse_rhythm(&hits).unwrap();
        assert_eq!(parsed.offsets, vec![0.0, 1.0, 1.5, 2.0]);
        assert!((parsed.total_beats - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_rhythm_4_quarters() {
        let hits: Vec<String> = vec!["1/4", "1/4", "1/4", "1/4"]
            .into_iter()
            .map(String::from)
            .collect();
        let parsed = parse_rhythm(&hits).unwrap();
        assert!((parsed.total_beats - 4.0).abs() < 1e-10);
        assert_eq!(parsed.offsets, vec![0.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_time_sig_parse() {
        assert_eq!(parse_time_sig("4/4").unwrap(), (4, 4));
        assert_eq!(parse_time_sig("3/4").unwrap(), (3, 4));
        assert_eq!(parse_time_sig("6/8").unwrap(), (6, 8));
    }

    #[test]
    fn test_beats_per_bar() {
        assert!((beats_per_bar(4, 4) - 4.0).abs() < 1e-10);
        assert!((beats_per_bar(3, 4) - 3.0).abs() < 1e-10);
        assert!((beats_per_bar(6, 8) - 3.0).abs() < 1e-10);
    }
}
