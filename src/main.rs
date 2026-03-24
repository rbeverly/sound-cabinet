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
        "generate" => cmd_generate(&args[2..])?,
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
    eprintln!("  sound-cabinet generate --pattern <file.yaml> --key <K> --mode <M> --chords \"...\"");
    eprintln!("                        --voice <name> [--range C2-G3] [--variations 5] [--seed 42] [-o out.sc]");
}

/// Parse a `--flag <value>` pair from args, returning the f64 value if present.
fn parse_flag_f64(args: &[String], flag: &str) -> Result<Option<f64>> {
    for (i, a) in args.iter().enumerate() {
        if a == flag {
            let val = args.get(i + 1)
                .ok_or_else(|| anyhow!("{flag} requires a number"))?;
            return Ok(Some(val.parse().map_err(|_| anyhow!("{flag} requires a number"))?));
        }
    }
    Ok(None)
}

/// Check if a string is the value argument to a --flag (i.e., the element after a -- flag).
fn is_flag_value(args: &[String], candidate: &str) -> bool {
    for (i, a) in args.iter().enumerate() {
        if a.starts_with("--") && a.len() > 2 {
            if let Some(next) = args.get(i + 1) {
                if next == candidate {
                    return true;
                }
            }
        }
    }
    false
}

/// Parse a `--flag <value>` pair from args, returning the string value if present.
fn parse_flag_str<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    for (i, a) in args.iter().enumerate() {
        if a == flag {
            return args.get(i + 1).map(|s| s.as_str());
        }
    }
    None
}

/// Parse a `--flag <value>` pair as u64.
fn parse_flag_u64(args: &[String], flag: &str) -> Result<Option<u64>> {
    for (i, a) in args.iter().enumerate() {
        if a == flag {
            let val = args.get(i + 1)
                .ok_or_else(|| anyhow!("{flag} requires a number"))?;
            return Ok(Some(val.parse().map_err(|_| anyhow!("{flag} requires a number"))?));
        }
    }
    Ok(None)
}

/// Algorithmic phrase generation from YAML pattern files.
fn cmd_generate(args: &[String]) -> Result<()> {
    let pattern_path = parse_flag_str(args, "--pattern")
        .ok_or_else(|| anyhow!("--pattern <file.yaml> is required"))?;
    let key = parse_flag_str(args, "--key")
        .ok_or_else(|| anyhow!("--key <note> is required (e.g., --key D)"))?;
    let mode = parse_flag_str(args, "--mode")
        .ok_or_else(|| anyhow!("--mode <mode> is required (e.g., --mode dorian)"))?;
    let chords = parse_flag_str(args, "--chords")
        .ok_or_else(|| anyhow!("--chords \"...\" is required (e.g., --chords \"Dm7 G7 Cmaj7\")"))?;
    let voice = parse_flag_str(args, "--voice")
        .ok_or_else(|| anyhow!("--voice <name> is required (e.g., --voice bass)"))?;

    let range = parse_flag_str(args, "--range").map(String::from);
    let variations = parse_flag_u64(args, "--variations")?.unwrap_or(5) as usize;
    let seed = parse_flag_u64(args, "--seed")?.unwrap_or_else(|| rand::random());
    let output = parse_flag_str(args, "-o").map(String::from);

    let config = sound_cabinet::generate::GenerateConfig {
        pattern_path: pattern_path.to_string(),
        key: key.to_string(),
        mode: mode.to_string(),
        chords: chords.to_string(),
        voice: voice.to_string(),
        range,
        variations,
        seed,
        output,
    };

    sound_cabinet::generate::run_generate(&config)
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
            | Command::MasterCeiling(_)
            | Command::MasterGain(_) => {
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
    let mut compress: Option<Vec<f64>> = None;
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
                    compress = Some(args[i].split(',')
                        .map(|s| s.parse::<f64>())
                        .collect::<std::result::Result<Vec<_>, _>>()
                        .map_err(|_| anyhow!("--compress: invalid numbers (e.g. --compress 1.0 or --compress -18,2,0.05,0.2)"))?);
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
    if let Some(params) = compress {
        match params.len() {
            1 => engine.set_master_compress(params[0] as f32),
            2 => engine.set_master_compress_params(params[0] as f32, params[1] as f32, 0.010, 0.200),
            4 => engine.set_master_compress_params(params[0] as f32, params[1] as f32, params[2], params[3]),
            _ => return Err(anyhow!("--compress: expected 1, 2, or 4 values")),
        }
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
        return Err(anyhow!("Usage: sound-cabinet play <score.sc> [-v] [--from <beat>]"));
    }

    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let from_beat = parse_flag_f64(args, "--from")?;
    let score_path = args.iter()
        .find(|a| !a.starts_with('-') && !is_flag_value(args, a))
        .ok_or_else(|| anyhow!("Usage: sound-cabinet play <score.sc> [-v] [--from <beat>]"))?;

    let mut engine = build_engine(score_path)?;
    engine.verbose = verbose;
    if let Some(beat) = from_beat {
        engine.skip_to_beat(beat);
        eprintln!("Skipping to beat {beat}...");
    }
    eprintln!("Playing{}... (Ctrl+C to stop)", if verbose { " (verbose)" } else { "" });
    realtime::play_realtime(engine)?;

    Ok(())
}

/// Watch a score file for changes and replay on save.
fn cmd_watch(args: &[String]) -> Result<()> {
    use notify::{RecursiveMode, Watcher};

    if args.is_empty() {
        return Err(anyhow!("Usage: sound-cabinet watch <score.sc> [-v] [--from <beat>]"));
    }

    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let from_beat = parse_flag_f64(args, "--from")?;
    let score_path = args.iter()
        .find(|a| !a.starts_with('-') && !is_flag_value(args, a))
        .ok_or_else(|| anyhow!("Usage: sound-cabinet watch <score.sc> [-v] [--from <beat>]"))?
        .clone();
    let watch_dir = Path::new(&score_path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    // Initial build
    let mut engine = build_engine(&score_path)?;
    engine.verbose = verbose;
    if let Some(beat) = from_beat {
        engine.skip_to_beat(beat);
    }
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
                    Ok(mut new_engine) => {
                        new_engine.verbose = verbose;
                        if let Some(beat) = from_beat {
                            new_engine.skip_to_beat(beat);
                        }
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
        return Err(anyhow!(
            "Usage: sound-cabinet piano <score.sc> [instrument-name] [--midi [port]]"
        ));
    }

    // Parse args: score path, optional instrument name, optional --midi flag
    let score_path = &args[0];
    let midi_flag = args.iter().any(|a| a == "--midi");
    let midi_port: Option<usize> = if midi_flag {
        // Check if --midi is followed by a number
        args.iter()
            .position(|a| a == "--midi")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse().ok())
    } else {
        None
    };
    let instrument_name = args
        .get(1)
        .filter(|a| !a.starts_with('-') && !is_flag_value(args, a))
        .map(|s| s.as_str());

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

    // Determine default note duration in beats
    let note_duration_beats = 2.0;

    // Try to open MIDI input.
    // IMPORTANT: _midi_conn must be kept alive — dropping it closes the connection.
    let (midi_rx, _midi_conn) = if midi_flag {
        match open_midi_input(midi_port) {
            Ok((rx, conn, port_name)) => {
                eprintln!("MIDI connected: {}", port_name);
                (Some(rx), Some(conn))
            }
            Err(e) => {
                eprintln!("MIDI: {} (keyboard only)", e);
                (None, None)
            }
        }
    } else {
        // Auto-detect: try to open, silently fall back to keyboard
        match open_midi_input(None) {
            Ok((rx, conn, port_name)) => {
                eprintln!("MIDI auto-detected: {}", port_name);
                (Some(rx), Some(conn))
            }
            Err(_) => (None, None),
        }
    };

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
    if midi_rx.is_some() {
        eprintln!("MIDI keyboard active — full range available.");
    }
    eprintln!();
    eprintln!("Press keys to play. Esc or Ctrl+C to quit.");

    // Enter raw terminal mode
    terminal::enable_raw_mode()?;

    // Helper closure to play a note given MIDI number and velocity
    let play_note =
        |engine: &Arc<Mutex<sound_cabinet::engine::Engine>>,
         midi: i32,
         velocity: f64,
         inst_name: Option<&str>| {
            let freq = midi_to_hz(midi);
            let name = midi_to_name(midi);

            // Clamp to safe range. Instruments often use lowpass(freq * N) which
            // pushes the filter cutoff above Nyquist and causes NaN. A 10 kHz limit
            // keeps freq*4 = 40 kHz still safe for coefficient calculation.
            if freq > 10000.0 || freq < 16.0 {
                eprint!("  {} ({:.1} Hz) — out of range, skipped\r\n", name, freq);
                return;
            }

            let expr = if let Some(inst_name) = inst_name {
                sound_cabinet::dsl::ast::Expr::FnCall {
                    name: inst_name.to_string(),
                    args: vec![sound_cabinet::dsl::ast::Expr::Number(freq)],
                }
            } else {
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

            // Apply velocity as gain: wrap expr in multiplication
            let expr = if (velocity - 1.0).abs() > 0.01 {
                sound_cabinet::dsl::ast::Expr::Mul(
                    Box::new(expr),
                    Box::new(sound_cabinet::dsl::ast::Expr::Number(velocity)),
                )
            } else {
                expr
            };

            let mut eng = engine.lock().unwrap();
            eng.handle_command_relative(Command::PlayAt {
                beat: 0.0,
                expr,
                duration_beats: note_duration_beats,
                source: None,
            })
            .unwrap_or_else(|e| {
                let _ = e;
            });

            eprint!("  {} ({:.1} Hz) vel {:.0}%\r\n", name, freq, velocity * 100.0);
        };

    let result = (|| -> Result<()> {
        loop {
            // Check MIDI events (non-blocking)
            if let Some(ref rx) = midi_rx {
                while let Ok((midi_note, velocity)) = rx.try_recv() {
                    if velocity > 0 {
                        // Note-on: velocity 1-127 mapped to 0.0-1.0
                        let vel = velocity as f64 / 127.0;
                        play_note(&engine, midi_note as i32, vel, instrument_name);
                    }
                    // Note-off (velocity 0) is ignored — decay handles note ending
                }
            }

            // Check keyboard events (50ms poll)
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                    if code == KeyCode::Esc
                        || (code == KeyCode::Char('c')
                            && modifiers.contains(KeyModifiers::CONTROL))
                    {
                        break;
                    }

                    if let KeyCode::Char(c) = code {
                        if let Some(midi) = key_to_midi(c) {
                            play_note(&engine, midi, 1.0, instrument_name);
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

/// Open a MIDI input connection. Returns a channel receiver for (note, velocity) events,
/// the connection handle (must be kept alive), and the port name.
fn open_midi_input(
    port_index: Option<usize>,
) -> Result<(
    crossbeam_channel::Receiver<(u8, u8)>,
    midir::MidiInputConnection<()>,
    String,
)> {
    let midi_in = midir::MidiInput::new("sound-cabinet")
        .map_err(|e| anyhow!("Cannot initialize MIDI: {e}"))?;

    let ports = midi_in.ports();
    if ports.is_empty() {
        return Err(anyhow!("No MIDI input devices found"));
    }

    // Select port
    let port = if let Some(idx) = port_index {
        ports
            .get(idx)
            .ok_or_else(|| {
                let names: Vec<String> = ports
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        format!("  {}: {}", i, midi_in.port_name(p).unwrap_or_default())
                    })
                    .collect();
                anyhow!(
                    "MIDI port {} not found. Available ports:\n{}",
                    idx,
                    names.join("\n")
                )
            })?
    } else {
        &ports[0]
    };

    let port_name = midi_in
        .port_name(port)
        .unwrap_or_else(|_| "Unknown".to_string());

    let (tx, rx) = crossbeam_channel::unbounded::<(u8, u8)>();

    // MIDI callback: parse note-on/note-off messages and send through channel
    let conn = midi_in
        .connect(
            port,
            "sound-cabinet-in",
            move |_timestamp, message, _data| {
                if message.len() >= 3 {
                    let status = message[0] & 0xF0;
                    let note = message[1];
                    let velocity = message[2];

                    match status {
                        0x90 => {
                            // Note-on (velocity 0 = note-off per MIDI spec)
                            let _ = tx.send((note, velocity));
                        }
                        0x80 => {
                            // Note-off
                            let _ = tx.send((note, 0));
                        }
                        _ => {} // Ignore CC, pitch bend, etc. for now
                    }
                }
            },
            (),
        )
        .map_err(|e| anyhow!("Cannot connect to MIDI port '{}': {e}", port_name))?;

    Ok((rx, conn, port_name))
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
                        source: None,
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
