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

/// A placement within a section.
#[derive(Debug, Clone)]
pub enum SectionEntry {
    /// `repeat boom_bap every 4 beats`
    RepeatEvery { name: String, every_beats: f64 },
    /// `play jazz_chords`
    Play { name: String },
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

/// A single command in the score.
#[derive(Debug, Clone)]
pub enum Command {
    /// Define a named voice: `voice pad = (saw(40) + sine(80))`
    VoiceDef { name: String, expr: Expr },
    /// Set tempo: `bpm 120`
    SetBpm(f64),
    /// Schedule playback: `at 0 play pad for 4 beats`
    PlayAt {
        beat: f64,
        expr: Expr,
        duration_beats: f64,
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
    /// Sustain pedal down: `pedal down at 4.0`
    PedalDown { beat: f64 },
    /// Sustain pedal up: `pedal up at 8.0`
    PedalUp { beat: f64 },
    /// Set swing amount: `swing 0.6` (0.5 = straight, 0.67 = triplet feel)
    SetSwing(f64),
    /// Set humanize jitter: `humanize 10` (±ms random timing offset)
    SetHumanize(f64),
}

/// A complete parsed score file.
#[derive(Debug, Clone)]
pub struct Script {
    pub commands: Vec<Command>,
}
