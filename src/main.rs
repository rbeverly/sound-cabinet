use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
        "watch" => cmd_watch(&args[2..])?,
        "piano" => cmd_piano(&args[2..])?,
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
    eprintln!("  sound-cabinet watch <score.sc>   (live reload on file save)");
    eprintln!("  sound-cabinet piano <score.sc>   (play with keyboard)");
    eprintln!("  sound-cabinet stream              (reads from stdin)");
}

/// Parse, import, and load only definitions (voices, instruments, fx, waves, bpm)
/// from a score file — no playback events scheduled.
fn load_definitions(score_path: &str) -> Result<Engine> {
    use sound_cabinet::dsl::Command;

    let source = std::fs::read_to_string(score_path)?;
    let script = parse_script(&source)?;
    let base_dir = Path::new(score_path).parent().unwrap_or(Path::new("."));
    let script = resolve_imports(script, base_dir)?;
    let script = expand_script(script, &mut rand::thread_rng())?;


    let mut engine = Engine::new(SAMPLE_RATE);
    for cmd in script.commands {
        match &cmd {
            Command::VoiceDef { .. }
            | Command::WaveDef { .. }
            | Command::SetBpm { .. }
            | Command::MasterCompress(_)
            | Command::MasterCeiling(_) => {
                engine.handle_command(cmd)?;
            }
            _ => {} // skip playback events, patterns, sections, etc.
        }
    }
    Ok(engine)
}

/// Parse, import, expand, and build an engine from a score file.
fn build_engine(score_path: &str) -> Result<Engine> {
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
    Ok(engine)
}

/// Render a score file to WAV.
fn cmd_render(args: &[String]) -> Result<()> {
    // Parse args: <score.sc> -o <output.wav> [--lufs <target>] [--compress <amount>] [--ceiling <dBFS>]
    let mut score_path: Option<&str> = None;
    let mut output_path: Option<PathBuf> = None;
    let mut target_lufs: Option<f64> = None;
    let mut compress: Option<f32> = None;
    let mut ceiling: Option<f32> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                if i < args.len() {
                    output_path = Some(PathBuf::from(&args[i]));
                }
            }
            "--lufs" => {
                i += 1;
                if i < args.len() {
                    target_lufs = Some(args[i].parse().map_err(|_| {
                        anyhow!("--lufs requires a number (e.g. --lufs -14)")
                    })?);
                }
            }
            "--compress" => {
                i += 1;
                if i < args.len() {
                    compress = Some(args[i].parse().map_err(|_| {
                        anyhow!("--compress requires a number (0.0 = off, 1.0 = default, 2.0 = heavy)")
                    })?);
                }
            }
            "--ceiling" => {
                i += 1;
                if i < args.len() {
                    ceiling = Some(args[i].parse().map_err(|_| {
                        anyhow!("--ceiling requires a number in dBFS (e.g. --ceiling -1.0)")
                    })?);
                }
            }
            _ => {
                if score_path.is_none() {
                    score_path = Some(&args[i]);
                }
            }
        }
        i += 1;
    }

    let usage = "Usage: sound-cabinet render <score.sc> -o <output.wav> [--lufs <target>] [--compress <amount>] [--ceiling <dBFS>]";
    let score_path = score_path.ok_or_else(|| anyhow!(usage))?;
    let output_path = output_path.ok_or_else(|| anyhow!(usage))?;

    let mut engine = build_engine(score_path)?;
    // CLI overrides for master bus
    if let Some(amount) = compress {
        engine.set_master_compress(amount);
    }
    if let Some(db) = ceiling {
        engine.set_master_ceiling(db);
    }
    render_to_wav(&mut engine, &output_path, target_lufs)?;
    eprintln!("Rendered to {}", output_path.display());

    Ok(())
}

/// Play a score file through speakers.
fn cmd_play(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(anyhow!("Usage: sound-cabinet play <score.sc>"));
    }

    let engine = build_engine(&args[0])?;
    eprintln!("Playing... (Ctrl+C to stop)");
    realtime::play_realtime(engine)?;

    Ok(())
}

/// Watch a score file for changes and replay on save.
fn cmd_watch(args: &[String]) -> Result<()> {
    use notify::{RecursiveMode, Watcher};

    if args.is_empty() {
        return Err(anyhow!("Usage: sound-cabinet watch <score.sc>"));
    }

    let score_path = args[0].clone();
    let watch_dir = Path::new(&score_path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    // Initial build
    let engine = build_engine(&score_path)?;
    let engine = Arc::new(Mutex::new(engine));

    // Start audio stream using play_streaming
    let (shutdown_tx, shutdown_rx) = crossbeam_channel::bounded::<()>(1);
    let engine_for_audio = Arc::clone(&engine);
    let audio_thread = thread::spawn(move || {
        if let Err(e) = realtime::play_streaming(engine_for_audio, shutdown_rx) {
            eprintln!("Audio error: {e}");
        }
    });

    // Set up file watcher
    let (fs_tx, fs_rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(fs_tx)?;
    watcher.watch(&watch_dir, RecursiveMode::Recursive)?;

    eprintln!("Watching {} (Ctrl+C to stop)", score_path);
    eprintln!("Playing...");

    loop {
        match fs_rx.recv() {
            Ok(Ok(event)) => {
                // Only react to .sc file changes
                let is_sc = event.paths.iter().any(|p| {
                    p.extension().map_or(false, |e| e == "sc")
                });
                if !is_sc {
                    continue;
                }

                // Debounce: editors often write temp files then rename.
                // Wait 200ms and drain any additional events.
                thread::sleep(Duration::from_millis(200));
                while fs_rx.try_recv().is_ok() {}

                // Rebuild engine
                eprintln!("\nFile changed, rebuilding...");
                match build_engine(&score_path) {
                    Ok(new_engine) => {
                        let mut eng = engine.lock().unwrap();
                        *eng = new_engine;
                        eprintln!("Playing...");
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        eprintln!("(keeping previous version)");
                    }
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {e}"),
            Err(_) => break, // channel closed
        }
    }

    let _ = shutdown_tx.send(());
    let _ = audio_thread.join();
    Ok(())
}

/// Map a keyboard key to a MIDI note number.
/// Layout:
///   Bottom row: Z S X D C V G B H N J M ,    → C3 to C4 (chromatic)
///   Top row:    Q 2 W 3 E R 5 T 6 Y 7 U I    → C4 to C5 (chromatic)
fn key_to_midi(c: char) -> Option<i32> {
    match c {
        // Bottom row: C3 (48) to C4 (60)
        'z' => Some(48),  // C3
        's' => Some(49),  // C#3
        'x' => Some(50),  // D3
        'd' => Some(51),  // D#3
        'c' => Some(52),  // E3
        'v' => Some(53),  // F3
        'g' => Some(54),  // F#3
        'b' => Some(55),  // G3
        'h' => Some(56),  // G#3
        'n' => Some(57),  // A3
        'j' => Some(58),  // A#3
        'm' => Some(59),  // B3
        ',' => Some(60),  // C4

        // Top row: C4 (60) to C5 (72)
        'q' => Some(60),  // C4
        '2' => Some(61),  // C#4
        'w' => Some(62),  // D4
        '3' => Some(63),  // D#4
        'e' => Some(64),  // E4
        'r' => Some(65),  // F4
        '5' => Some(66),  // F#4
        't' => Some(67),  // G4
        '6' => Some(68),  // G#4
        'y' => Some(69),  // A4
        '7' => Some(70),  // A#4
        'u' => Some(71),  // B4
        'i' => Some(72),  // C5

        _ => None,
    }
}

/// Convert MIDI note number to frequency in Hz (A4 = 440).
fn midi_to_hz(midi: i32) -> f64 {
    440.0 * 2.0_f64.powf((midi as f64 - 69.0) / 12.0)
}

/// Note name from MIDI number (for display).
fn midi_to_name(midi: i32) -> String {
    let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (midi / 12) - 1;
    let note = midi % 12;
    format!("{}{}", names[note as usize], octave)
}

/// Live piano mode: play an instrument with your keyboard.
fn cmd_piano(args: &[String]) -> Result<()> {
    use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
    use crossterm::terminal;
    use sound_cabinet::dsl::Command;

    if args.is_empty() {
        return Err(anyhow!("Usage: sound-cabinet piano <score.sc> [instrument-name]"));
    }

    let score_path = &args[0];
    let instrument_name = args.get(1).map(|s| s.as_str());

    // Load definitions from the score file
    let engine = load_definitions(score_path)?;

    // Wrap in Arc<Mutex> for audio thread
    let engine = Arc::new(Mutex::new(engine));

    // Start audio stream
    let (shutdown_tx, shutdown_rx) = crossbeam_channel::bounded::<()>(1);
    let engine_for_audio = Arc::clone(&engine);
    let audio_thread = thread::spawn(move || {
        if let Err(e) = realtime::play_streaming(engine_for_audio, shutdown_rx) {
            eprintln!("Audio error: {e}");
        }
    });

    // Determine default note duration in beats (notes play for this long then decay)
    let note_duration_beats = 2.0;

    eprintln!("Piano mode — {}", score_path);
    if let Some(name) = instrument_name {
        eprintln!("Instrument: {}", name);
    } else {
        eprintln!("Using first available instrument/voice (or sine fallback)");
    }
    eprintln!();
    eprintln!("  │ s │ d │   │ g │ h │ j │   │    │ 2 │ 3 │   │ 5 │ 6 │ 7 │   │");
    eprintln!("  │C#3│D#3│   │F#3│G#3│A#3│   │    │C#4│D#4│   │F#4│G#4│A#4│   │");
    eprintln!("┌─┴┬──┴┬──┴┬──┴┬──┴┬──┴┬──┴┬──┤  ┌─┴┬──┴┬──┴┬──┴┬──┴┬──┴┬──┴┬──┤");
    eprintln!("│ z│ x │ c │ v │ b │ n │ m │ ,│  │ q│ w │ e │ r │ t │ y │ u │ i│");
    eprintln!("│C3│D3 │E3 │F3 │G3 │A3 │B3 │C4│  │C4│D4 │E4 │F4 │G4 │A4 │B4 │C5│");
    eprintln!("└──┴───┴───┴───┴───┴───┴───┴──┘  └──┴───┴───┴───┴───┴───┴───┴──┘");
    eprintln!();
    eprintln!("Press keys to play. Esc or Ctrl+C to quit.");

    // Enter raw terminal mode
    terminal::enable_raw_mode()?;

    let result = (|| -> Result<()> {
        loop {
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                    // Exit on Esc or Ctrl+C
                    if code == KeyCode::Esc
                        || (code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL))
                    {
                        break;
                    }

                    if let KeyCode::Char(c) = code {
                        if let Some(midi) = key_to_midi(c) {
                            let freq = midi_to_hz(midi);
                            let name = midi_to_name(midi);

                            // Build the expression for this note
                            let expr = if let Some(inst_name) = instrument_name {
                                // instrument(freq)
                                sound_cabinet::dsl::ast::Expr::FnCall {
                                    name: inst_name.to_string(),
                                    args: vec![sound_cabinet::dsl::ast::Expr::Number(freq)],
                                }
                            } else {
                                // Default: sine(freq) with decay
                                sound_cabinet::dsl::ast::Expr::Pipe(
                                    Box::new(sound_cabinet::dsl::ast::Expr::FnCall {
                                        name: "sine".to_string(),
                                        args: vec![sound_cabinet::dsl::ast::Expr::Number(freq)],
                                    }),
                                    Box::new(sound_cabinet::dsl::ast::Expr::FnCall {
                                        name: "decay".to_string(),
                                        args: vec![sound_cabinet::dsl::ast::Expr::Number(6.0)],
                                    }),
                                )
                            };

                            // Schedule the note relative to current playback position
                            let mut eng = engine.lock().unwrap();
                            eng.handle_command_relative(Command::PlayAt {
                                beat: 0.0,
                                expr,
                                duration_beats: note_duration_beats,
                            })
                            .unwrap_or_else(|e| {
                                // Can't easily print in raw mode, just ignore
                                let _ = e;
                            });

                            // Print the note name (raw mode needs \r\n)
                            eprint!("  {} ({:.1} Hz)\r\n", name, freq);
                        }
                    }
                }
            }
        }
        Ok(())
    })();

    // Restore terminal
    terminal::disable_raw_mode()?;
    eprintln!("\nPiano mode ended.");

    let _ = shutdown_tx.send(());
    let _ = audio_thread.join();

    result
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
                    eng.handle_command(sound_cabinet::dsl::Command::VoiceDef { name, expr, kind: sound_cabinet::dsl::ast::DefKind::Voice })
                        .unwrap_or_else(|e| eprintln!("Engine error: {e}"));
                }
                EngineMsg::SetBpm(bpm) => {
                    let mut eng = engine_for_msgs.lock().unwrap();
                    eng.handle_command(sound_cabinet::dsl::Command::SetBpm { bpm, at_beat: None })
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
