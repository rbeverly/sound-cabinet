use std::collections::HashMap;

/// DSP expression tree — the core recursive type representing signal graphs.
#[derive(Debug, Clone)]
pub enum Expr {
    /// A literal number (e.g., 440.0, 0.5)
    Number(f64),
    /// A function call (e.g., sine(440), lowpass(2000, 0.7))
    FnCall { name: String, args: Vec<Expr> },
    /// A reference to a named voice definition
    VoiceRef(String),
    /// Pipe/chain: a >> b (output of a feeds into b)
    Pipe(Box<Expr>, Box<Expr>),
    /// Sum/mix: a + b
    Sum(Box<Expr>, Box<Expr>),
    /// Scale/multiply: a * b (typically number * signal or signal * number)
    Mul(Box<Expr>, Box<Expr>),
    /// Divide: a / b (for inverse frequency scaling like 1000 / freq)
    Div(Box<Expr>, Box<Expr>),
    /// Subtract: a - b
    Sub(Box<Expr>, Box<Expr>),
    /// Range/sweep: start -> end (linear interpolation over event duration)
    Range(f64, f64),
}

/// A single event within a pattern — beat offset is relative to pattern start.
#[derive(Debug, Clone)]
pub struct PatternEvent {
    pub beat_offset: f64,
    pub expr: Expr,
    pub duration_beats: f64,
}

/// Voice substitution map: maps pattern voice names to actual voice/instrument names.
pub type WithMap = HashMap<String, String>;

/// A placement within a section.
#[derive(Debug, Clone)]
pub enum SectionEntry {
    /// `repeat boom_bap every 4 beats [with {kick = 808}]`
    RepeatEvery { name: String, every_beats: f64, with_map: Option<WithMap> },
    /// `play jazz_chords [with {melody = rhodes}]`
    Play { name: String, with_map: Option<WithMap> },
}

/// A weighted choice for `pick`.
#[derive(Debug, Clone)]
pub struct WeightedChoice {
    pub name: String,
    pub weight: f64,
}

/// An item inside a `repeat N { ... }` block.
#[derive(Debug, Clone)]
pub enum RepeatBody {
    Play(String),
    Pick(Vec<WeightedChoice>),
    Shuffle(Vec<String>),
}

/// What kind of named definition this is.
/// Preserved from parse time for validation and error messages.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DefKind {
    /// `voice name = expr` — complete signal graph, used directly
    Voice,
    /// `fx name = expr` — effect chain (1-in, 1-out), used in pipe chains
    Fx,
    /// `instrument name = expr` — template with `freq` variable, instantiated with a frequency
    Instrument,
}

impl std::fmt::Display for DefKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DefKind::Voice => write!(f, "voice"),
            DefKind::Fx => write!(f, "fx"),
            DefKind::Instrument => write!(f, "instrument"),
        }
    }
}

/// A single command in the score.
#[derive(Debug, Clone)]
pub enum Command {
    /// Define a named voice, fx chain, or instrument
    VoiceDef { name: String, expr: Expr, kind: DefKind },
    /// Set tempo: `bpm 120`. Optional beat position for mid-score tempo changes.
    SetBpm { bpm: f64, at_beat: Option<f64> },
    /// Schedule playback: `at 0 play pad for 4 beats`
    PlayAt {
        beat: f64,
        expr: Expr,
        duration_beats: f64,
        /// Provenance: which pattern/section/voice produced this event (for verbose output).
        source: Option<String>,
        /// The as-played voice name, preserved across `with` substitution.
        /// E.g., if `with duelingpiano1 = piano`, this is "duelingpiano1" even
        /// though `expr` resolves to the `piano` instrument.
        voice_label: Option<String>,
    },
    /// Import another .sc file: `import voices/kick.sc`
    Import { path: String },
    /// Define a named pattern with relative events
    PatternDef {
        name: String,
        duration_beats: f64,
        events: Vec<PatternEvent>,
        swing: Option<f64>,
        humanize: Option<f64>,
    },
    /// Define a named section that composes patterns
    SectionDef {
        name: String,
        duration_beats: f64,
        entries: Vec<SectionEntry>,
        with_map: Option<WithMap>,
    },
    /// Top-level sequential play: `play intro`
    PlaySequential { name: String },
    /// Repeat block: `repeat 4 { ... }`
    RepeatBlock {
        count: u32,
        body: Vec<RepeatBody>,
    },
    /// Define a named wavetable: `wave wonky = [0.0, 0.3, ...]`
    WaveDef { name: String, samples: Vec<f64> },
    /// Sustain pedal down: `pedal down at 4.0` or `pedal down piano at 4.0`
    /// or `pedal down [piano, strings] at 4.0`
    PedalDown { beat: f64, voices: Vec<String> },
    /// Sustain pedal up: `pedal up at 4.0` or `pedal up piano at 4.0`
    PedalUp { beat: f64, voices: Vec<String> },
    /// Normalize an instrument's volume: `normalize bass 0.5`
    /// The engine renders test tones through the instrument at multiple frequencies,
    /// measures average RMS, and applies a correction gain so the instrument
    /// produces output at the target level (0.0-1.0 scale, where 1.0 = 0 dBFS).
    Normalize { name: String, target: f64 },
    /// Set swing amount: `swing 0.6` (0.5 = straight, 0.67 = triplet feel)
    SetSwing(f64),
    /// Set humanize jitter: `humanize 10` (±ms random timing offset)
    SetHumanize(f64),
    /// Set global voice bindings: `with kick = analog_kick, snare = tight_snare`
    SetWith(WithMap),
    /// Master bus compression:
    /// - `master compress 0.5` — amount (0.0 = off, 1.0 = default, 2.0 = heavy)
    /// - `master compress -18 2` — threshold (dB), ratio
    /// - `master compress -18 2 0.05 0.2` — threshold, ratio, attack (s), release (s)
    MasterCompress(Vec<f64>),
    /// Master bus limiter ceiling: `master ceiling -1.0` (dBFS)
    MasterCeiling(f64),
    /// Master output gain in dB: `master gain -6` (applied before compressor/limiter)
    MasterGain(f64),
}

/// A complete parsed score file.
#[derive(Debug, Clone)]
pub struct Script {
    pub commands: Vec<Command>,
}
