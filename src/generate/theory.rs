//! Music theory primitives: pitch, scales, modes, chords, and note naming.
//!
//! This module provides the foundational types for algorithmic phrase generation
//! and is designed to be reusable across the codebase (instrument generation, etc.).

use anyhow::{anyhow, Result};

// ---------------------------------------------------------------------------
// Pitch
// ---------------------------------------------------------------------------

/// A concrete pitch represented as a MIDI note number.
/// Middle C (C4) = 60, A4 = 69.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pitch(pub i32);

impl Pitch {
    pub fn from_midi(midi: i32) -> Self {
        Pitch(midi)
    }

    pub fn midi(self) -> i32 {
        self.0
    }

    pub fn to_hz(self) -> f64 {
        440.0 * 2.0_f64.powf((self.0 as f64 - 69.0) / 12.0)
    }

    /// Convert a frequency in Hz to the nearest MIDI pitch.
    pub fn from_hz(hz: f64) -> Self {
        if hz <= 0.0 {
            return Pitch(0);
        }
        let midi = (69.0 + 12.0 * (hz / 440.0).log2()).round() as i32;
        Pitch(midi.max(0).min(127))
    }

    /// Parse a note name like "C4", "Bb3", "F#5" into a Pitch.
    pub fn from_note_name(s: &str) -> Result<Self> {
        let mut chars = s.chars();
        let letter = chars.next().ok_or_else(|| anyhow!("Empty note name"))?;
        let semitone_base: i32 = match letter {
            'C' => 0,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' => 11,
            _ => return Err(anyhow!("Invalid note letter: {letter}")),
        };

        let rest: String = chars.collect();
        let (accidental, octave_str) = if rest.starts_with('#') || rest.starts_with('s') {
            (1i32, &rest[1..])
        } else if rest.starts_with('b') {
            (-1i32, &rest[1..])
        } else {
            (0i32, rest.as_str())
        };

        let octave: i32 = octave_str
            .parse()
            .map_err(|_| anyhow!("Invalid octave in note: {s}"))?;

        Ok(Pitch((octave + 1) * 12 + semitone_base + accidental))
    }

    /// Convert to a note name string, using key-aware enharmonic spelling.
    /// If no key context, uses sharps by default.
    pub fn to_note_name(self, key: Option<PitchClass>) -> String {
        let midi = self.0;
        let octave = (midi / 12) - 1;
        let pc = midi.rem_euclid(12);

        let name = if let Some(root) = key {
            preferred_spelling(root, pc)
        } else {
            // Default: sharps
            match pc {
                0 => "C",
                1 => "C#",
                2 => "D",
                3 => "D#",
                4 => "E",
                5 => "F",
                6 => "F#",
                7 => "G",
                8 => "G#",
                9 => "A",
                10 => "A#",
                11 => "B",
                _ => unreachable!(),
            }
        };
        format!("{name}{octave}")
    }

    /// Transpose up by N semitones.
    pub fn transpose(self, semitones: i32) -> Self {
        Pitch(self.0 + semitones)
    }

    /// Pitch class (0-11) where C=0.
    pub fn pitch_class(self) -> i32 {
        self.0.rem_euclid(12)
    }

    /// Octave number (C4 -> 4).
    pub fn octave(self) -> i32 {
        (self.0 / 12) - 1
    }
}

// ---------------------------------------------------------------------------
// PitchClass
// ---------------------------------------------------------------------------

/// One of the 12 chromatic pitch classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PitchClass {
    C,
    Cs,
    D,
    Eb,
    E,
    F,
    Fs,
    G,
    Ab,
    A,
    Bb,
    B,
}

impl PitchClass {
    /// Parse from string: "C", "C#", "Db", "Bb", "F#", etc.
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "C" => Ok(PitchClass::C),
            "C#" | "Cs" => Ok(PitchClass::Cs),
            "Db" => Ok(PitchClass::Eb), // Db is enharmonic with C#, but we'll use Cs
            "D" => Ok(PitchClass::D),
            "D#" | "Ds" => Ok(PitchClass::Eb),
            "Eb" => Ok(PitchClass::Eb),
            "E" => Ok(PitchClass::E),
            "F" => Ok(PitchClass::F),
            "F#" | "Fs" => Ok(PitchClass::Fs),
            "Gb" => Ok(PitchClass::Fs),
            "G" => Ok(PitchClass::G),
            "G#" | "Gs" => Ok(PitchClass::Ab),
            "Ab" => Ok(PitchClass::Ab),
            "A" => Ok(PitchClass::A),
            "A#" | "As" => Ok(PitchClass::Bb),
            "Bb" => Ok(PitchClass::Bb),
            "B" => Ok(PitchClass::B),
            "Cb" => Ok(PitchClass::B),
            _ => Err(anyhow!("Unknown pitch class: {s}")),
        }
    }

    /// Semitone value (C=0, C#=1, ..., B=11).
    pub fn semitone(self) -> i32 {
        match self {
            PitchClass::C => 0,
            PitchClass::Cs => 1,
            PitchClass::D => 2,
            PitchClass::Eb => 3,
            PitchClass::E => 4,
            PitchClass::F => 5,
            PitchClass::Fs => 6,
            PitchClass::G => 7,
            PitchClass::Ab => 8,
            PitchClass::A => 9,
            PitchClass::Bb => 10,
            PitchClass::B => 11,
        }
    }

    /// Build a Pitch at the given octave.
    pub fn at_octave(self, octave: i32) -> Pitch {
        Pitch((octave + 1) * 12 + self.semitone())
    }
}

/// Key-aware enharmonic spelling. Given the key root pitch class and a
/// chromatic pitch class (0-11), returns the preferred note name string.
fn preferred_spelling(key: PitchClass, pc: i32) -> &'static str {
    // Keys that prefer flats vs sharps
    let use_flats = matches!(
        key,
        PitchClass::F
            | PitchClass::Bb
            | PitchClass::Eb
            | PitchClass::Ab
    );

    match pc {
        0 => "C",
        1 => {
            if use_flats {
                "Db"
            } else {
                "C#"
            }
        }
        2 => "D",
        3 => {
            if use_flats {
                "Eb"
            } else {
                "D#"
            }
        }
        4 => "E",
        5 => "F",
        6 => {
            if use_flats {
                "Gb"
            } else {
                "F#"
            }
        }
        7 => "G",
        8 => {
            if use_flats {
                "Ab"
            } else {
                "G#"
            }
        }
        9 => "A",
        10 => {
            if use_flats {
                "Bb"
            } else {
                "A#"
            }
        }
        11 => "B",
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Mode
// ---------------------------------------------------------------------------

/// Musical modes, each defined by their interval pattern (semitones from root).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Major,      // Ionian
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Minor,      // Aeolian / Natural minor
    Locrian,
    HarmonicMinor,
    MelodicMinor,
    PentatonicMajor,
    PentatonicMinor,
    Blues,
}

impl Mode {
    /// Parse mode name from string (case-insensitive).
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "major" | "ionian" => Ok(Mode::Major),
            "dorian" => Ok(Mode::Dorian),
            "phrygian" => Ok(Mode::Phrygian),
            "lydian" => Ok(Mode::Lydian),
            "mixolydian" => Ok(Mode::Mixolydian),
            "minor" | "aeolian" | "natural_minor" => Ok(Mode::Minor),
            "locrian" => Ok(Mode::Locrian),
            "harmonic_minor" => Ok(Mode::HarmonicMinor),
            "melodic_minor" => Ok(Mode::MelodicMinor),
            "pentatonic_major" | "pentatonic" => Ok(Mode::PentatonicMajor),
            "pentatonic_minor" => Ok(Mode::PentatonicMinor),
            "blues" => Ok(Mode::Blues),
            _ => Err(anyhow!("Unknown mode: {s}")),
        }
    }

    /// Semitone intervals from root within one octave.
    pub fn intervals(self) -> &'static [i32] {
        match self {
            Mode::Major => &[0, 2, 4, 5, 7, 9, 11],
            Mode::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Mode::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
            Mode::Lydian => &[0, 2, 4, 6, 7, 9, 11],
            Mode::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            Mode::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Mode::Locrian => &[0, 1, 3, 5, 6, 8, 10],
            Mode::HarmonicMinor => &[0, 2, 3, 5, 7, 8, 11],
            Mode::MelodicMinor => &[0, 2, 3, 5, 7, 9, 11],
            Mode::PentatonicMajor => &[0, 2, 4, 7, 9],
            Mode::PentatonicMinor => &[0, 3, 5, 7, 10],
            Mode::Blues => &[0, 3, 5, 6, 7, 10],
        }
    }

    /// Number of scale degrees per octave.
    pub fn degree_count(self) -> usize {
        self.intervals().len()
    }
}

// ---------------------------------------------------------------------------
// Scale
// ---------------------------------------------------------------------------

/// A scale: a root pitch class + a mode.
#[derive(Debug, Clone, Copy)]
pub struct Scale {
    pub root: PitchClass,
    pub mode: Mode,
}

impl Scale {
    pub fn new(root: PitchClass, mode: Mode) -> Self {
        Scale { root, mode }
    }

    /// Get the pitch for a given scale degree (0-based) at a given octave.
    /// Degree 0 = root. Degrees can be negative or exceed the mode's degree count
    /// (wraps across octaves).
    pub fn degree_to_pitch(&self, degree: i32, octave: i32) -> Pitch {
        let intervals = self.mode.intervals();
        let n = intervals.len() as i32;

        // Handle wrapping: degree 7 in a 7-note scale = degree 0, octave+1
        let (oct_offset, deg) = if degree >= 0 {
            (degree / n, (degree % n) as usize)
        } else {
            // For negative degrees, adjust so we wrap correctly
            let adj = (-degree + n - 1) / n; // ceiling division
            let shifted = degree + adj * n;
            (degree / n - if degree % n != 0 { 1 } else { 0 }, shifted.rem_euclid(n) as usize)
        };

        let semitone = self.root.semitone() + intervals[deg];
        Pitch((octave + 1 + oct_offset) * 12 + semitone)
    }

    /// Find the scale degree and octave of a pitch, or None if not in scale.
    pub fn pitch_to_degree(&self, pitch: Pitch) -> Option<(i32, i32)> {
        let pc = pitch.pitch_class();
        let root_semi = self.root.semitone();
        let interval = (pc - root_semi).rem_euclid(12);
        let intervals = self.mode.intervals();

        for (deg, &iv) in intervals.iter().enumerate() {
            if iv == interval {
                let octave = pitch.octave();
                // Adjust octave if the note's pitch class is below the root
                let oct = if pc < root_semi {
                    octave - 1
                } else {
                    octave
                };
                // Hmm, this needs more care. Let's use a simpler approach:
                // The degree's pitch at this octave:
                let candidate = self.degree_to_pitch(deg as i32, oct);
                if candidate == pitch {
                    return Some((deg as i32, oct));
                }
                // Try octave+1
                let candidate2 = self.degree_to_pitch(deg as i32, oct + 1);
                if candidate2 == pitch {
                    return Some((deg as i32, oct + 1));
                }
                // Try octave-1
                let candidate3 = self.degree_to_pitch(deg as i32, oct - 1);
                if candidate3 == pitch {
                    return Some((deg as i32, oct - 1));
                }
            }
        }
        None
    }

    /// Move one diatonic step up from the given pitch.
    /// If the pitch is in the scale, moves to the next degree.
    /// If not in the scale, snaps to the nearest scale tone above.
    pub fn step_up(&self, pitch: Pitch) -> Pitch {
        if let Some((deg, oct)) = self.pitch_to_degree(pitch) {
            self.degree_to_pitch(deg + 1, oct)
        } else {
            // Snap to nearest scale tone above
            self.nearest_above(pitch)
        }
    }

    /// Move one diatonic step down from the given pitch.
    pub fn step_down(&self, pitch: Pitch) -> Pitch {
        if let Some((deg, oct)) = self.pitch_to_degree(pitch) {
            self.degree_to_pitch(deg - 1, oct)
        } else {
            self.nearest_below(pitch)
        }
    }

    /// Leap N diatonic steps from the given pitch.
    /// Positive = up, negative = down.
    pub fn leap(&self, pitch: Pitch, steps: i32) -> Pitch {
        if let Some((deg, oct)) = self.pitch_to_degree(pitch) {
            self.degree_to_pitch(deg + steps, oct)
        } else {
            // Snap to nearest, then leap
            let snapped = if steps > 0 {
                self.nearest_above(pitch)
            } else {
                self.nearest_below(pitch)
            };
            if let Some((deg, oct)) = self.pitch_to_degree(snapped) {
                // Subtract 1 from steps since snapping already moved one step
                self.degree_to_pitch(deg + steps - steps.signum(), oct)
            } else {
                // Fallback: chromatic
                pitch.transpose(steps * 2)
            }
        }
    }

    /// Find the nearest scale tone at or above the given pitch.
    fn nearest_above(&self, pitch: Pitch) -> Pitch {
        for offset in 0..=12 {
            let candidate = Pitch(pitch.0 + offset);
            if self.pitch_to_degree(candidate).is_some() {
                return candidate;
            }
        }
        pitch // fallback
    }

    /// Find the nearest scale tone at or below the given pitch.
    fn nearest_below(&self, pitch: Pitch) -> Pitch {
        for offset in 0..=12 {
            let candidate = Pitch(pitch.0 - offset);
            if self.pitch_to_degree(candidate).is_some() {
                return candidate;
            }
        }
        pitch // fallback
    }

    /// Get all scale tones in a pitch range (inclusive).
    pub fn pitches_in_range(&self, low: Pitch, high: Pitch) -> Vec<Pitch> {
        let mut result = Vec::new();
        let intervals = self.mode.intervals();
        let root_semi = self.root.semitone();

        // Iterate through MIDI range
        for midi in low.0..=high.0 {
            let pc = midi.rem_euclid(12);
            let interval = (pc - root_semi).rem_euclid(12);
            if intervals.contains(&interval) {
                result.push(Pitch(midi));
            }
        }
        result
    }
}

// ---------------------------------------------------------------------------
// Chord
// ---------------------------------------------------------------------------

/// A parsed chord with root, quality intervals, and octave.
#[derive(Debug, Clone)]
pub struct Chord {
    pub root: PitchClass,
    pub intervals: Vec<i32>,
    pub octave: i32,
}

impl Chord {
    /// Parse a chord name like "Dm7", "Cmaj7", "G7", "Am".
    /// Octave defaults to 3 for bass-friendly voicing.
    pub fn parse(name: &str) -> Result<Self> {
        let mut chars = name.chars().peekable();

        let letter = chars.next().ok_or_else(|| anyhow!("Empty chord name"))?;
        let semitone_base: i32 = match letter {
            'C' => 0,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' => 11,
            _ => return Err(anyhow!("Invalid chord root: {letter}")),
        };

        let rest: String = chars.collect();
        let (accidental, rest) = if rest.starts_with('#') || rest.starts_with('s') {
            (1i32, &rest[1..])
        } else if rest.starts_with('b') {
            (-1i32, &rest[1..])
        } else {
            (0i32, rest.as_str())
        };

        let root_semi = (semitone_base + accidental).rem_euclid(12);
        let root = pitch_class_from_semitone(root_semi);

        // Quality lookup (longest match first)
        let qualities: &[(&str, &[i32])] = &[
            ("mmaj7", &[0, 3, 7, 11]),        // minor-major 7th
            ("m7b5", &[0, 3, 6, 10]),         // half-diminished
            ("maj9", &[0, 4, 7, 11, 14]),
            ("min9", &[0, 3, 7, 10, 14]),
            ("add9", &[0, 4, 7, 14]),         // major add 9
            ("maj7", &[0, 4, 7, 11]),
            ("min7", &[0, 3, 7, 10]),
            ("dim7", &[0, 3, 6, 9]),
            ("aug7", &[0, 4, 8, 10]),
            ("dom9", &[0, 4, 7, 10, 14]),
            ("dom7", &[0, 4, 7, 10]),
            ("sus2", &[0, 2, 7]),
            ("sus4", &[0, 5, 7]),
            ("min", &[0, 3, 7]),
            ("maj", &[0, 4, 7]),
            ("dim", &[0, 3, 6]),
            ("aug", &[0, 4, 8]),
            ("m9", &[0, 3, 7, 10, 14]),
            ("m7", &[0, 3, 7, 10]),
            ("m6", &[0, 3, 7, 9]),            // minor 6th
            ("m", &[0, 3, 7]),
            ("9", &[0, 4, 7, 10, 14]),
            ("7", &[0, 4, 7, 10]),
            ("6", &[0, 4, 7, 9]),             // major 6th
        ];

        let (intervals, after_quality) = qualities
            .iter()
            .find_map(|(suffix, ivs)| {
                rest.strip_prefix(suffix).map(|remainder| (ivs.to_vec(), remainder))
            })
            .unwrap_or_else(|| (vec![0, 4, 7], rest)); // default: major triad

        // Parse optional octave
        let octave: i32 = if after_quality.is_empty() {
            3 // default octave for chords
        } else {
            after_quality
                .parse()
                .map_err(|_| anyhow!("Invalid octave in chord: {name}"))
                .unwrap_or(3)
        };

        Ok(Chord {
            root,
            intervals,
            octave,
        })
    }

    /// Root pitch at the chord's octave.
    pub fn root_pitch(&self) -> Pitch {
        self.root.at_octave(self.octave)
    }

    /// All chord tones at the chord's octave, sorted by pitch.
    pub fn tones(&self) -> Vec<Pitch> {
        let base = self.root.at_octave(self.octave);
        self.intervals.iter().map(|&iv| Pitch(base.0 + iv)).collect()
    }

    /// Chord tones within a pitch range, may span multiple octaves.
    pub fn tones_in_range(&self, low: Pitch, high: Pitch) -> Vec<Pitch> {
        let mut result = Vec::new();
        // Try multiple octaves
        for oct in 0..=8 {
            let base = self.root.at_octave(oct);
            for &iv in &self.intervals {
                let p = Pitch(base.0 + iv);
                if p >= low && p <= high {
                    result.push(p);
                }
            }
        }
        result.sort();
        result.dedup();
        result
    }
}

fn pitch_class_from_semitone(s: i32) -> PitchClass {
    match s.rem_euclid(12) {
        0 => PitchClass::C,
        1 => PitchClass::Cs,
        2 => PitchClass::D,
        3 => PitchClass::Eb,
        4 => PitchClass::E,
        5 => PitchClass::F,
        6 => PitchClass::Fs,
        7 => PitchClass::G,
        8 => PitchClass::Ab,
        9 => PitchClass::A,
        10 => PitchClass::Bb,
        11 => PitchClass::B,
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Range
// ---------------------------------------------------------------------------

/// Instrument pitch range.
#[derive(Debug, Clone, Copy)]
pub struct PitchRange {
    pub low: Pitch,
    pub high: Pitch,
}

impl PitchRange {
    pub fn new(low: Pitch, high: Pitch) -> Self {
        PitchRange { low, high }
    }

    /// Parse "C2-G3" or "C2:G3" into a range.
    pub fn parse(s: &str) -> Result<Self> {
        // Find the separator. For '-', we need the last one preceded by a digit
        // to handle note names like "C#2-G3".
        let sep_idx = if s.contains(':') {
            s.find(':').unwrap()
        } else {
            // Find '-' that's preceded by a digit (i.e., the range separator, not an accidental)
            let mut found = None;
            for (i, c) in s.char_indices() {
                if c == '-' && i > 0 {
                    let prev = s.as_bytes()[i - 1];
                    if prev.is_ascii_digit() {
                        found = Some(i);
                        break;
                    }
                }
            }
            found.ok_or_else(|| anyhow!("Range must be in format 'C2-G3' or 'C2:G3': '{s}'"))?
        };

        let low = Pitch::from_note_name(&s[..sep_idx])?;
        let high = Pitch::from_note_name(&s[sep_idx + 1..])?;
        Ok(PitchRange { low, high })
    }

    /// Clamp a pitch into this range by octave transposition.
    pub fn clamp(&self, pitch: Pitch) -> Pitch {
        let mut p = pitch;
        while p.0 < self.low.0 {
            p = Pitch(p.0 + 12);
        }
        while p.0 > self.high.0 {
            p = Pitch(p.0 - 12);
        }
        // If still out of range after transposition (range < 1 octave),
        // pick whichever bound is closer
        if p.0 < self.low.0 || p.0 > self.high.0 {
            if (pitch.0 - self.low.0).abs() < (pitch.0 - self.high.0).abs() {
                self.low
            } else {
                self.high
            }
        } else {
            p
        }
    }

    pub fn contains(&self, pitch: Pitch) -> bool {
        pitch.0 >= self.low.0 && pitch.0 <= self.high.0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_round_trip() {
        for name in &["C4", "A4", "D2", "G#3", "Bb5", "E1"] {
            let p = Pitch::from_note_name(name).unwrap();
            let back = p.to_note_name(None);
            // Re-parse should give same MIDI
            let p2 = Pitch::from_note_name(&back).unwrap();
            assert_eq!(p.midi(), p2.midi(), "Round-trip failed for {name}: got {back}");
        }
    }

    #[test]
    fn test_a4_is_440() {
        let p = Pitch::from_note_name("A4").unwrap();
        assert_eq!(p.midi(), 69);
        assert!((p.to_hz() - 440.0).abs() < 0.01);
    }

    #[test]
    fn test_middle_c() {
        let p = Pitch::from_note_name("C4").unwrap();
        assert_eq!(p.midi(), 60);
    }

    #[test]
    fn test_scale_d_dorian_degrees() {
        let scale = Scale::new(PitchClass::D, Mode::Dorian);
        // D dorian: D E F G A B C
        let d2 = scale.degree_to_pitch(0, 2);
        assert_eq!(d2, Pitch::from_note_name("D2").unwrap());

        let e2 = scale.degree_to_pitch(1, 2);
        assert_eq!(e2, Pitch::from_note_name("E2").unwrap());

        let f2 = scale.degree_to_pitch(2, 2);
        assert_eq!(f2, Pitch::from_note_name("F2").unwrap()); // F natural, not F#

        let g2 = scale.degree_to_pitch(3, 2);
        assert_eq!(g2, Pitch::from_note_name("G2").unwrap());
    }

    #[test]
    fn test_scale_step_up_down() {
        let scale = Scale::new(PitchClass::C, Mode::Major);
        let c4 = Pitch::from_note_name("C4").unwrap();
        let d4 = scale.step_up(c4);
        assert_eq!(d4, Pitch::from_note_name("D4").unwrap());

        let back = scale.step_down(d4);
        assert_eq!(back, c4);
    }

    #[test]
    fn test_scale_step_wraps_octave() {
        let scale = Scale::new(PitchClass::C, Mode::Major);
        let b4 = Pitch::from_note_name("B4").unwrap();
        let c5 = scale.step_up(b4);
        assert_eq!(c5, Pitch::from_note_name("C5").unwrap());
    }

    #[test]
    fn test_scale_leap() {
        let scale = Scale::new(PitchClass::C, Mode::Major);
        let c4 = Pitch::from_note_name("C4").unwrap();
        // Leap 4 steps up = C D E F G -> G4 (degree 4)
        let g4 = scale.leap(c4, 4);
        assert_eq!(g4, Pitch::from_note_name("G4").unwrap());
    }

    #[test]
    fn test_chord_parse() {
        let chord = Chord::parse("Dm7").unwrap();
        assert_eq!(chord.root, PitchClass::D);
        assert_eq!(chord.intervals, vec![0, 3, 7, 10]);
    }

    #[test]
    fn test_chord_tones() {
        let chord = Chord::parse("Cmaj").unwrap();
        let tones = chord.tones();
        // C3, E3, G3 (octave defaults to 3)
        assert_eq!(tones.len(), 3);
        assert_eq!(tones[0], PitchClass::C.at_octave(3));
        assert_eq!(tones[1], Pitch(PitchClass::C.at_octave(3).0 + 4)); // E
        assert_eq!(tones[2], Pitch(PitchClass::C.at_octave(3).0 + 7)); // G
    }

    #[test]
    fn test_range_clamp() {
        let range = PitchRange::new(
            Pitch::from_note_name("C2").unwrap(),
            Pitch::from_note_name("G3").unwrap(),
        );
        // C5 should be clamped down to C3 or C2
        let clamped = range.clamp(Pitch::from_note_name("C5").unwrap());
        assert!(range.contains(clamped));

        // Already in range
        let d2 = Pitch::from_note_name("D2").unwrap();
        assert_eq!(range.clamp(d2), d2);
    }

    #[test]
    fn test_enharmonic_flat_key() {
        // In key of F (1 flat), Bb should be spelled Bb not A#
        let bb = Pitch::from_note_name("Bb3").unwrap();
        let name = bb.to_note_name(Some(PitchClass::F));
        assert_eq!(name, "Bb3");
    }

    #[test]
    fn test_enharmonic_sharp_key() {
        // In key of D (2 sharps), F# should be spelled F# not Gb
        let fs = Pitch::from_note_name("F#3").unwrap();
        let name = fs.to_note_name(Some(PitchClass::D));
        assert_eq!(name, "F#3");
    }

    #[test]
    fn test_pitches_in_range() {
        let scale = Scale::new(PitchClass::C, Mode::Major);
        let range_low = Pitch::from_note_name("C4").unwrap();
        let range_high = Pitch::from_note_name("C5").unwrap();
        let pitches = scale.pitches_in_range(range_low, range_high);
        // C D E F G A B C = 8 pitches
        assert_eq!(pitches.len(), 8);
    }
}
