use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};

use crate::dsl::ast::Command;
use crate::dsl::parser::parse_line;
use crate::stream::channel::EngineMsg;

/// Receive raw text lines, parse them, and dispatch EngineMsg commands.
pub fn run_dispatcher(rx: Receiver<String>, tx: Sender<EngineMsg>) -> Result<()> {
    for line in rx {
        match parse_line(&line) {
            Ok(cmd) => {
                let msg = command_to_msg(cmd);
                if tx.send(msg).is_err() {
                    break; // receiver dropped
                }
            }
            Err(e) => {
                eprintln!("Parse error: {e}");
            }
        }
    }
    // Signal shutdown when input ends
    let _ = tx.send(EngineMsg::Shutdown);
    Ok(())
}

fn command_to_msg(cmd: Command) -> EngineMsg {
    match cmd {
        Command::VoiceDef { name, expr, .. } => EngineMsg::DefineVoice { name, expr },
        Command::SetBpm { bpm, .. } => EngineMsg::SetBpm(bpm),
        Command::PlayAt {
            beat,
            expr,
            duration_beats,
            ..
        } => EngineMsg::PlayNow {
            beat_offset: beat,
            expr,
            duration_beats,
        },
        // Streaming mode only handles single-line commands
        _ => return EngineMsg::SetBpm(0.0), // no-op fallback
    }
}
