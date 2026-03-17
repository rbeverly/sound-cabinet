use std::io::{self, BufRead};

use anyhow::Result;
use crossbeam_channel::Sender;

/// Read lines from stdin and send them to the dispatcher channel.
/// This is trivially replaceable with any other line source (HTTP, WebSocket, LLM API).
pub fn run_stdin_reader(tx: Sender<String>) -> Result<()> {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if tx.send(trimmed).is_err() {
            break; // receiver dropped
        }
    }
    Ok(())
}
