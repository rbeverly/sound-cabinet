use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{anyhow, Result};

use sound_cabinet::dsl::{expand_script, parse_script, resolve_imports};
use sound_cabinet::engine::Engine;
use sound_cabinet::render::realtime;
use sound_cabinet::render::wav::render_to_wav;
use sound_cabinet::stream::channel::EngineMsg;
use sound_cabinet::stream::dispatcher::run_dispatcher;
use sound_cabinet::stream::stdin_reader::run_stdin_reader;

const SAMPLE_RATE: f64 = 44100.0;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "render" => cmd_render(&args[2..])?,
        "play" => cmd_play(&args[2..])?,
        "stream" => cmd_stream()?,
        _ => {
            print_usage();
            return Err(anyhow!("Unknown command: {}", args[1]));
        }
    }

    Ok(())
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  sound-cabinet render <score.sc> -o <output.wav>");
    eprintln!("  sound-cabinet play <score.sc>");
    eprintln!("  sound-cabinet stream   (reads from stdin)");
}

/// Render a score file to WAV.
fn cmd_render(args: &[String]) -> Result<()> {
    if args.len() < 3 || args[1] != "-o" {
        return Err(anyhow!("Usage: sound-cabinet render <score.sc> -o <output.wav>"));
    }

    let score_path = &args[0];
    let output_path = PathBuf::from(&args[2]);

    let source = std::fs::read_to_string(score_path)?;
    let script = parse_script(&source)?;
    let base_dir = Path::new(score_path).parent().unwrap_or(Path::new("."));
    let script = resolve_imports(script, base_dir)?;
    let script = expand_script(script, &mut rand::thread_rng())?;

    let mut engine = Engine::new(SAMPLE_RATE);
    for cmd in script.commands {
        engine.handle_command(cmd)?;
    }
    engine.apply_pedal();

    render_to_wav(&mut engine, &output_path)?;
    eprintln!("Rendered to {}", output_path.display());

    Ok(())
}

/// Play a score file through speakers.
fn cmd_play(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(anyhow!("Usage: sound-cabinet play <score.sc>"));
    }

    let score_path = &args[0];
    let source = std::fs::read_to_string(score_path)?;
    let script = parse_script(&source)?;
    let base_dir = Path::new(score_path).parent().unwrap_or(Path::new("."));
    let script = resolve_imports(script, base_dir)?;
    let script = expand_script(script, &mut rand::thread_rng())?;

    let mut engine = Engine::new(SAMPLE_RATE);
    for cmd in script.commands {
        engine.handle_command(cmd)?;
    }
    engine.apply_pedal();

    eprintln!("Playing... (Ctrl+C to stop)");
    realtime::play_realtime(engine)?;

    Ok(())
}

/// Stream mode: read lines from stdin, play in real-time.
fn cmd_stream() -> Result<()> {
    let engine = Arc::new(Mutex::new(Engine::new(SAMPLE_RATE)));

    // Channels: stdin_reader -> dispatcher -> engine
    let (line_tx, line_rx) = crossbeam_channel::unbounded::<String>();
    let (msg_tx, msg_rx) = crossbeam_channel::unbounded::<EngineMsg>();
    let (shutdown_tx, shutdown_rx) = crossbeam_channel::bounded::<()>(1);

    let engine_for_audio = Arc::clone(&engine);

    // Thread 1: stdin reader
    let reader_handle = thread::spawn(move || {
        if let Err(e) = run_stdin_reader(line_tx) {
            eprintln!("Reader error: {e}");
        }
    });

    // Thread 2: dispatcher (parse + send)
    let dispatcher_handle = thread::spawn(move || {
        if let Err(e) = run_dispatcher(line_rx, msg_tx) {
            eprintln!("Dispatcher error: {e}");
        }
    });

    // Thread 3: message consumer (drains EngineMsg into the engine)
    let engine_for_msgs = Arc::clone(&engine);
    let msg_handle = thread::spawn(move || {
        for msg in msg_rx {
            match msg {
                EngineMsg::Shutdown => {
                    let _ = shutdown_tx.send(());
                    break;
                }
                EngineMsg::DefineVoice { name, expr } => {
                    let mut eng = engine_for_msgs.lock().unwrap();
                    eng.handle_command(sound_cabinet::dsl::Command::VoiceDef { name, expr })
                        .unwrap_or_else(|e| eprintln!("Engine error: {e}"));
                }
                EngineMsg::SetBpm(bpm) => {
                    let mut eng = engine_for_msgs.lock().unwrap();
                    eng.handle_command(sound_cabinet::dsl::Command::SetBpm(bpm))
                        .unwrap_or_else(|e| eprintln!("Engine error: {e}"));
                }
                EngineMsg::PlayNow {
                    beat_offset,
                    expr,
                    duration_beats,
                } => {
                    let mut eng = engine_for_msgs.lock().unwrap();
                    eng.handle_command_relative(sound_cabinet::dsl::Command::PlayAt {
                        beat: beat_offset,
                        expr,
                        duration_beats,
                    })
                    .unwrap_or_else(|e| eprintln!("Engine error: {e}"));
                }
            }
        }
    });

    eprintln!("Streaming mode. Type score lines, Ctrl+D to end.");

    // Start audio playback on this thread
    realtime::play_streaming(engine_for_audio, shutdown_rx)?;

    reader_handle.join().ok();
    dispatcher_handle.join().ok();
    msg_handle.join().ok();

    Ok(())
}
