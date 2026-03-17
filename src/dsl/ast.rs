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
}

/// A complete parsed score file.
#[derive(Debug, Clone)]
pub struct Script {
    pub commands: Vec<Command>,
}
