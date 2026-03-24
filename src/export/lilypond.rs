//! LilyPond notation writer.
//!
//! Converts extracted NoteEvents into LilyPond (.ly) format for sheet music
//! rendering. Groups by voice, assigns clefs, quantizes to grid, fills rests,
//! and inserts bar lines.

use std::collections::BTreeMap;

use crate::generate::theory::{Pitch, PitchClass};

use super::extract::{NoteEvent, TempoEvent};
use super::ExportConfig;

/// Generate a complete LilyPond file from extracted note events.
pub fn write_lilypond(
    events: &[NoteEvent],
    tempos: &[TempoEvent],
    config: &ExportConfig,
) -> String {
    let beats_per_bar = parse_beats_per_bar(&config.time_sig);
    let bpm = tempos.first().map(|t| t.bpm).unwrap_or(120.0);

    // Group events by voice name
    let mut voices: BTreeMap<String, Vec<&NoteEvent>> = BTreeMap::new();
    for ev in events {
        voices.entry(ev.voice_name.clone()).or_default().push(ev);
    }

    let mut out = String::new();

    // Header
    out.push_str("\\version \"2.24.0\"\n\n");
    out.push_str("\\header {\n");
    if let Some(ref title) = config.title {
        out.push_str(&format!("  title = \"{title}\"\n"));
    }
    out.push_str("}\n\n");

    // Score block
    out.push_str("\\score {\n  <<\n");

    for (voice_name, voice_events) in &voices {
        let staff = render_staff(voice_name, voice_events, bpm, beats_per_bar, config);
        out.push_str(&staff);
    }

    out.push_str("  >>\n");
    out.push_str("  \\layout { }\n");
    out.push_str("  \\midi { }\n");
    out.push_str("}\n");

    out
}

/// Render a single staff for one voice.
fn render_staff(
    voice_name: &str,
    events: &[&NoteEvent],
    bpm: f64,
    beats_per_bar: f64,
    config: &ExportConfig,
) -> String {
    let is_percussion = events.iter().all(|e| e.pitch.is_none());

    if is_percussion {
        return render_percussion_staff(voice_name, events, bpm, beats_per_bar, config);
    }

    // Determine clef from pitch range
    let pitches: Vec<i32> = events
        .iter()
        .filter_map(|e| e.pitch.map(|p| p.midi()))
        .collect();
    let median_midi = if pitches.is_empty() {
        60
    } else {
        let mut sorted = pitches.clone();
        sorted.sort();
        sorted[sorted.len() / 2]
    };
    let clef = if median_midi < 60 { "bass" } else { "treble" };

    let mut out = String::new();
    let display_name = capitalize(voice_name);
    out.push_str(&format!(
        "    \\new Staff \\with {{ instrumentName = \"{}\" }} {{\n",
        display_name
    ));
    out.push_str(&format!("      \\clef {clef}\n"));

    // Key signature
    if let Some(ref key) = config.key {
        if let Some(ly_key) = key_to_lilypond(key) {
            out.push_str(&format!("      {ly_key}\n"));
        }
    }

    // Time signature
    out.push_str(&format!("      \\time {}\n", config.time_sig));

    // Tempo
    out.push_str(&format!("      \\tempo 4 = {}\n", bpm as u32));

    // Render note sequence
    out.push_str("      ");
    let note_str = render_note_sequence(events, beats_per_bar, config);
    out.push_str(&note_str);
    out.push('\n');

    out.push_str("    }\n");
    out
}

/// Render a percussion staff.
fn render_percussion_staff(
    voice_name: &str,
    events: &[&NoteEvent],
    bpm: f64,
    beats_per_bar: f64,
    config: &ExportConfig,
) -> String {
    let mut out = String::new();
    let display_name = capitalize(voice_name);
    out.push_str(&format!(
        "    \\new DrumStaff \\with {{ instrumentName = \"{}\" }} {{\n",
        display_name
    ));
    out.push_str(&format!("      \\time {}\n", config.time_sig));
    out.push_str(&format!("      \\tempo 4 = {}\n", bpm as u32));
    out.push_str("      \\drummode {\n        ");

    // Map voice names to LilyPond drum names
    let drum_name = match voice_name {
        "kick" | "bd" => "bd",
        "snare" | "sd" => "sn",
        "hat" | "hihat" | "hh" => "hh",
        "tom" | "tomh" => "tomh",
        "crash" => "cymc",
        "ride" => "cymr",
        other => other,
    };

    let note_str = render_drum_sequence(events, drum_name, beats_per_bar);
    out.push_str(&note_str);
    out.push('\n');
    out.push_str("      }\n");
    out.push_str("    }\n");
    out
}

/// Render a sequence of pitched notes with rests and bar lines.
fn render_note_sequence(
    events: &[&NoteEvent],
    beats_per_bar: f64,
    _config: &ExportConfig,
) -> String {
    if events.is_empty() {
        return String::new();
    }

    // Find the total duration we need to fill
    let last_end = events
        .iter()
        .map(|e| e.beat + e.duration_beats)
        .fold(0.0_f64, f64::max);
    let total_bars = (last_end / beats_per_bar).ceil() as usize;
    let total_beats = total_bars as f64 * beats_per_bar;

    // Quantize events to 16th note grid
    let grid = 0.25; // 16th note
    let mut quantized: Vec<(f64, f64, Option<Pitch>)> = events
        .iter()
        .map(|e| {
            let beat = quantize(e.beat, grid);
            let dur = quantize(e.duration_beats, grid).max(grid);
            (beat, dur, e.pitch)
        })
        .collect();
    quantized.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // Build the note sequence with rests filling gaps
    let mut result = Vec::new();
    let mut cursor = 0.0;

    for &(beat, dur, pitch) in &quantized {
        // Fill gap with rests
        if beat > cursor + 0.001 {
            let gap = beat - cursor;
            result.extend(make_rests(gap, cursor, beats_per_bar));
            cursor = beat;
        }

        // Split note across bar lines if needed
        let bar_remaining = beats_per_bar - (cursor % beats_per_bar);
        if dur > bar_remaining + 0.001 && bar_remaining < beats_per_bar - 0.001 {
            // Note spans a bar line — tie it
            if let Some(p) = pitch {
                let ly_pitch = pitch_to_lilypond(p);
                let dur1 = duration_to_lilypond(bar_remaining);
                let dur2 = duration_to_lilypond(dur - bar_remaining);
                result.push(format!("{ly_pitch}{dur1}~"));
                // Insert bar line
                result.push("|".to_string());
                result.push(format!("{ly_pitch}{dur2}"));
            }
        } else if let Some(p) = pitch {
            let ly_pitch = pitch_to_lilypond(p);
            let ly_dur = duration_to_lilypond(dur);
            result.push(format!("{ly_pitch}{ly_dur}"));
        }

        cursor = beat + dur;

        // Check for bar line
        let bar_pos = cursor % beats_per_bar;
        if bar_pos.abs() < 0.001 || (bar_pos - beats_per_bar).abs() < 0.001 {
            result.push("|".to_string());
        }
    }

    // Fill remaining beats to complete the last bar
    if cursor < total_beats - 0.001 {
        let gap = total_beats - cursor;
        result.extend(make_rests(gap, cursor, beats_per_bar));
    }

    result.join(" ")
}

/// Render a drum sequence.
fn render_drum_sequence(
    events: &[&NoteEvent],
    drum_name: &str,
    beats_per_bar: f64,
) -> String {
    if events.is_empty() {
        return String::new();
    }

    let last_end = events
        .iter()
        .map(|e| e.beat + e.duration_beats)
        .fold(0.0_f64, f64::max);
    let total_bars = (last_end / beats_per_bar).ceil() as usize;
    let total_beats = total_bars as f64 * beats_per_bar;

    let grid = 0.25;
    let mut quantized: Vec<(f64, f64)> = events
        .iter()
        .map(|e| {
            let beat = quantize(e.beat, grid);
            let dur = quantize(e.duration_beats, grid).max(grid);
            (beat, dur)
        })
        .collect();
    quantized.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let mut result = Vec::new();
    let mut cursor = 0.0;

    for &(beat, dur) in &quantized {
        if beat > cursor + 0.001 {
            let gap = beat - cursor;
            result.extend(make_rests(gap, cursor, beats_per_bar));
            cursor = beat;
        }

        let ly_dur = duration_to_lilypond(dur);
        result.push(format!("{drum_name}{ly_dur}"));
        cursor = beat + dur;

        let bar_pos = cursor % beats_per_bar;
        if bar_pos.abs() < 0.001 || (bar_pos - beats_per_bar).abs() < 0.001 {
            result.push("|".to_string());
        }
    }

    if cursor < total_beats - 0.001 {
        let gap = total_beats - cursor;
        result.extend(make_rests(gap, cursor, beats_per_bar));
    }

    result.join(" ")
}

/// Create rest tokens to fill a gap of `beats` duration.
fn make_rests(beats: f64, _cursor: f64, _beats_per_bar: f64) -> Vec<String> {
    let mut remaining = beats;
    let mut rests = Vec::new();

    // Decompose into standard rest durations (largest first)
    let standard_durations = [4.0, 3.0, 2.0, 1.5, 1.0, 0.75, 0.5, 0.25];
    for &d in &standard_durations {
        while remaining >= d - 0.001 {
            let ly_dur = duration_to_lilypond(d);
            rests.push(format!("r{ly_dur}"));
            remaining -= d;
            if remaining < 0.001 {
                break;
            }
        }
    }

    rests
}

/// Convert a Pitch to LilyPond notation.
/// LilyPond: c = C3, c' = C4 (middle C), c'' = C5, c, = C2, c,, = C1
/// Sharps: cis, dis, fis, gis, ais. Flats: ces, ees, bes, etc.
pub fn pitch_to_lilypond(pitch: Pitch) -> String {
    let midi = pitch.midi();
    let pc = midi.rem_euclid(12);
    let octave = (midi / 12) - 1; // MIDI octave: C4=60 → octave 4

    let note_name = match pc {
        0 => "c",
        1 => "cis",
        2 => "d",
        3 => "ees",
        4 => "e",
        5 => "f",
        6 => "fis",
        7 => "g",
        8 => "aes",
        9 => "a",
        10 => "bes",
        11 => "b",
        _ => unreachable!(),
    };

    // c,,, = C0, c,, = C1, c, = C2, c = C3, c' = C4, c'' = C5, etc.
    let octave_mark = match octave {
        0 => ",,,",
        1 => ",,",
        2 => ",",
        3 => "",   // unadorned = octave 3
        4 => "'",
        5 => "''",
        6 => "'''",
        7 => "''''",
        _ => {
            return if octave < 3 {
                format!("{}{}", note_name, ",".repeat((3 - octave) as usize))
            } else {
                format!("{}{}", note_name, "'".repeat((octave - 3) as usize))
            };
        }
    };

    format!("{note_name}{octave_mark}")
}

/// Convert a beat duration to LilyPond duration string.
/// 4.0 = whole note (1), 2.0 = half (2), 1.0 = quarter (4),
/// 0.5 = eighth (8), 0.25 = sixteenth (16).
/// Dotted: 1.5 = dotted quarter (4.), 0.75 = dotted eighth (8.).
pub fn duration_to_lilypond(beats: f64) -> String {
    // Match common durations
    let quantized = (beats * 16.0).round() / 16.0; // snap to 64th note

    match () {
        _ if close(quantized, 4.0) => "1".to_string(),
        _ if close(quantized, 3.0) => "2.".to_string(),
        _ if close(quantized, 2.0) => "2".to_string(),
        _ if close(quantized, 1.5) => "4.".to_string(),
        _ if close(quantized, 1.0) => "4".to_string(),
        _ if close(quantized, 0.75) => "8.".to_string(),
        _ if close(quantized, 0.5) => "8".to_string(),
        _ if close(quantized, 0.375) => "16.".to_string(),
        _ if close(quantized, 0.25) => "16".to_string(),
        _ if close(quantized, 0.125) => "32".to_string(),
        _ => {
            // For odd durations, find the closest standard duration
            if quantized >= 2.0 {
                "2".to_string()
            } else if quantized >= 1.0 {
                "4".to_string()
            } else if quantized >= 0.5 {
                "8".to_string()
            } else {
                "16".to_string()
            }
        }
    }
}

/// Convert a key string like "Am", "D", "Bb", "F#m" to LilyPond key command.
fn key_to_lilypond(key: &str) -> Option<String> {
    let is_minor = key.ends_with('m') && !key.ends_with("dim") && !key.ends_with("maj");
    let root_str = if is_minor {
        &key[..key.len() - 1]
    } else {
        key
    };

    let pc = PitchClass::parse(root_str).ok()?;
    let ly_note = match pc {
        PitchClass::C => "c",
        PitchClass::Cs => "cis",
        PitchClass::D => "d",
        PitchClass::Eb => "ees",
        PitchClass::E => "e",
        PitchClass::F => "f",
        PitchClass::Fs => "fis",
        PitchClass::G => "g",
        PitchClass::Ab => "aes",
        PitchClass::A => "a",
        PitchClass::Bb => "bes",
        PitchClass::B => "b",
    };

    let mode = if is_minor { "\\minor" } else { "\\major" };
    Some(format!("\\key {ly_note} {mode}"))
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

fn quantize(value: f64, grid: f64) -> f64 {
    (value / grid).round() * grid
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn parse_beats_per_bar(time_sig: &str) -> f64 {
    if let Some((num, denom)) = time_sig.split_once('/') {
        if let (Ok(n), Ok(d)) = (num.parse::<f64>(), denom.parse::<f64>()) {
            return n * (4.0 / d);
        }
    }
    4.0 // default to 4/4
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_to_lilypond_middle_c() {
        let p = Pitch::from_midi(60); // C4
        assert_eq!(pitch_to_lilypond(p), "c'");
    }

    #[test]
    fn test_pitch_to_lilypond_a4() {
        let p = Pitch::from_midi(69); // A4
        assert_eq!(pitch_to_lilypond(p), "a'");
    }

    #[test]
    fn test_pitch_to_lilypond_c3() {
        let p = Pitch::from_midi(48); // C3
        assert_eq!(pitch_to_lilypond(p), "c");
    }

    #[test]
    fn test_pitch_to_lilypond_c2() {
        let p = Pitch::from_midi(36); // C2
        assert_eq!(pitch_to_lilypond(p), "c,");
    }

    #[test]
    fn test_pitch_to_lilypond_sharp() {
        let p = Pitch::from_midi(61); // C#4
        assert_eq!(pitch_to_lilypond(p), "cis'");
    }

    #[test]
    fn test_pitch_to_lilypond_flat() {
        let p = Pitch::from_midi(70); // Bb4
        assert_eq!(pitch_to_lilypond(p), "bes'");
    }

    #[test]
    fn test_duration_to_lilypond() {
        assert_eq!(duration_to_lilypond(4.0), "1");
        assert_eq!(duration_to_lilypond(2.0), "2");
        assert_eq!(duration_to_lilypond(1.0), "4");
        assert_eq!(duration_to_lilypond(0.5), "8");
        assert_eq!(duration_to_lilypond(0.25), "16");
        assert_eq!(duration_to_lilypond(1.5), "4.");
        assert_eq!(duration_to_lilypond(0.75), "8.");
    }

    #[test]
    fn test_key_to_lilypond() {
        assert_eq!(
            key_to_lilypond("Am"),
            Some("\\key a \\minor".to_string())
        );
        assert_eq!(
            key_to_lilypond("D"),
            Some("\\key d \\major".to_string())
        );
        assert_eq!(
            key_to_lilypond("Bb"),
            Some("\\key bes \\major".to_string())
        );
        assert_eq!(
            key_to_lilypond("F#m"),
            Some("\\key fis \\minor".to_string())
        );
    }

    #[test]
    fn test_quantize() {
        assert!((quantize(0.48, 0.25) - 0.5).abs() < 0.01);
        assert!((quantize(1.02, 0.25) - 1.0).abs() < 0.01);
        assert!((quantize(2.0, 0.25) - 2.0).abs() < 0.01);
    }
}
