use crate::dsl::ast::Expr;

/// Messages sent from the dispatcher to the audio engine.
#[derive(Debug, Clone)]
pub enum EngineMsg {
    /// Define or redefine a named voice.
    DefineVoice { name: String, expr: Expr },
    /// Set the tempo.
    SetBpm(f64),
    /// Play a voice starting now (beat offset relative to current position).
    PlayNow {
        beat_offset: f64,
        expr: Expr,
        duration_beats: f64,
    },
    /// Graceful shutdown.
    Shutdown,
}
