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
        "profile" => cmd_profile(&args[2..])?,
        "export" => cmd_export(&args[2..])?,
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
    eprintln!("  sound-cabinet profile <score.sc>    (analyze per-voice levels)");
    eprintln!("  sound-cabinet generate --pattern <file.yaml> --key <K> --mode <M> --chords \"...\"");
    eprintln!("                        --voice <name> [--range C2-G3] [--variations 5] [--seed 42] [-o out.sc]");
    eprintln!("  sound-cabinet export <score.sc> -o out.ly [--voice piano] [--source verse_a]");
    eprintln!("                        [--format pdf] [--time 4/4] [--key Am] [--from 0 --to 32]");
}

/// Print per-voice level summary from engine stats.
fn print_voice_levels(engine: &sound_cabinet::engine::Engine) {
    let levels = engine.voice_levels();
    if levels.is_empty() {
        return;
    }

    // Sort by RMS level descending (loudest first)
    let mut entries: Vec<_> = levels.iter().collect();
    entries.sort_by(|a, b| b.1.rms_db().partial_cmp(&a.1.rms_db()).unwrap());

    eprintln!();
    eprintln!("  {:<20} {:>10} {:>10}  {}", "Voice", "RMS", "Peak", "Status");
    eprintln!("  {:<20} {:>10} {:>10}  {}", "─────", "───", "────", "──────");

    for (name, stats) in &entries {
        let rms = stats.rms_db();
        let peak = stats.peak_db();

        let status = if rms < -60.0 {
            "INAUDIBLE"
        } else if rms < -40.0 {
            "Very quiet"
        } else if rms < -24.0 {
            "Quiet"
        } else if peak > -1.0 {
            "Dominant"
        } else {
            "OK"
        };

        eprintln!(
            "  {:<20} {:>7.1} dB {:>7.1} dB  {}",
            name, rms, peak, status
        );
    }
    eprintln!();
}

/// Profile a score: render each voice in isolation and report levels.
fn cmd_profile(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(anyhow!("Usage: sound-cabinet profile <score.sc>"));
    }

    let score_path = args.iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| anyhow!("Usage: sound-cabinet profile <score.sc>"))?;

    eprintln!("Profiling {}...", score_path);

    // First, do a full render to get combined levels
    let mut engine = build_engine(score_path)?;

    // Render all samples (discard audio, just accumulate stats)
    let mut buffer = vec![0.0f32; 1024];
    while !engine.is_finished() {
        buffer.fill(0.0);
        engine.render_samples(&mut buffer);
    }
    let _ = engine.flush_master();

    let levels = engine.voice_levels();
    if levels.is_empty() {
        eprintln!("No voiced events found in the score.");
        return Ok(());
    }

    // Sort by RMS level descending
    let mut entries: Vec<_> = levels.iter().collect();
    entries.sort_by(|a, b| b.1.rms_db().partial_cmp(&a.1.rms_db()).unwrap());

    // Find the loudest voice for relative comparison
    let max_rms = entries.first().map(|(_, s)| s.rms_db()).unwrap_or(-100.0);

    eprintln!();
    eprintln!("  {:<20} {:>10} {:>10} {:>10}  {}", "Voice", "RMS", "Peak", "Relative", "Status");
    eprintln!("  {:<20} {:>10} {:>10} {:>10}  {}", "─────", "───", "────", "────────", "──────");

    for (name, stats) in &entries {
        let rms = stats.rms_db();
        let peak = stats.peak_db();
        let relative = rms - max_rms;

        let status = if rms < -60.0 {
            "INAUDIBLE — probably can't hear this"
        } else if rms < -40.0 {
            "Very quiet — likely masked by louder voices"
        } else if rms < -24.0 {
            "Quiet"
        } else if peak > -1.0 {
            "Dominant — may clip"
        } else if relative > -3.0 {
            "Loudest"
        } else {
            "OK"
        };

        eprintln!(
            "  {:<20} {:>7.1} dB {:>7.1} dB {:>+7.1} dB  {}",
            name, rms, peak, relative, status
        );
    }

    // Check for large gaps between loudest and quietest
    let min_rms = entries.last().map(|(_, s)| s.rms_db()).unwrap_or(-100.0);
    let gap = max_rms - min_rms;
    if gap > 30.0 && min_rms > -100.0 {
        eprintln!();
        eprintln!("  Warning: {:.0} dB gap between loudest and quietest voice.", gap);
        eprintln!("  The quietest voices are likely inaudible in the mix.");
    }

    eprintln!();
    Ok(())
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
/// Export sheet music in LilyPond format.
fn cmd_export(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(anyhow!(
            "Usage: sound-cabinet export <score.sc> -o <output.ly> [--voice V] [--source S] [--format pdf]"
        ));
    }

    let score_path = args
        .iter()
        .find(|a| !a.starts_with('-') && !is_flag_value(args, a))
        .ok_or_else(|| anyhow!("Score file path required"))?
        .clone();

    let output = parse_flag_str(args, "-o")
        .ok_or_else(|| anyhow!("-o <output> is required"))?
        .to_string();

    let format_str = parse_flag_str(args, "--format").unwrap_or("lilypond");
    let format = match format_str {
        "pdf" => sound_cabinet::export::ExportFormat::Pdf,
        "ly" | "lilypond" => sound_cabinet::export::ExportFormat::Lilypond,
        _ => return Err(anyhow!("Unknown format: {format_str} (use 'lilypond' or 'pdf')")),
    };

    // Auto-detect format from output extension
    let format = if output.ends_with(".pdf") && format_str == "lilypond" {
        sound_cabinet::export::ExportFormat::Pdf
    } else {
        format
    };

    let config = sound_cabinet::export::ExportConfig {
        score_path,
        output,
        format,
        voice_filter: parse_flag_str(args, "--voice").map(String::from),
        source_filter: parse_flag_str(args, "--source").map(String::from),
        from_beat: parse_flag_f64(args, "--from")?,
        to_beat: parse_flag_f64(args, "--to")?,
        time_sig: parse_flag_str(args, "--time")
            .unwrap_or("4/4")
            .to_string(),
        key: parse_flag_str(args, "--key").map(String::from),
        title: parse_flag_str(args, "--title").map(String::from),
    };

    sound_cabinet::export::run_export(&config)
}

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
            "--solo" => {
                // handled below
                i += 1;
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
    // Apply solo filter
    if let Some(voice) = parse_flag_str(args, "--solo") {
        let voices: Vec<String> = voice.split(',').map(|s| s.trim().to_string()).collect();
        eprintln!("Solo: {}", voices.join(", "));
        engine.set_solo(voices);
    }
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
    print_voice_levels(&engine);
    eprintln!("Rendered to {}", output_path.display());

    Ok(())
}

/// Play a score file through speakers.
fn cmd_play(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(anyhow!("Usage: sound-cabinet play <score.sc> [-v] [--from <beat>] [--solo <voice>]"));
    }

    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let show_vu = args.iter().any(|a| a == "--vu" || a == "--meters");
    let from_beat = parse_flag_f64(args, "--from")?;
    let solo = parse_flag_str(args, "--solo");
    let score_path = args.iter()
        .find(|a| !a.starts_with('-') && !is_flag_value(args, a))
        .ok_or_else(|| anyhow!("Usage: sound-cabinet play <score.sc> [-v] [--vu] [--from <beat>] [--solo <voice>]"))?;

    let mut engine = build_engine(score_path)?;
    engine.verbose = verbose;
    if let Some(voice) = solo {
        let voices: Vec<String> = voice.split(',').map(|s| s.trim().to_string()).collect();
        eprintln!("Solo: {}", voices.join(", "));
        engine.set_solo(voices);
    }
    if let Some(beat) = from_beat {
        engine.skip_to_beat(beat);
        eprintln!("Skipping to beat {beat}...");
    }
    eprintln!("Playing{}... (Ctrl+C to stop)", if verbose { " (verbose)" } else { "" });
    if show_vu {
        realtime::play_realtime_vu(engine)?;
    } else {
        realtime::play_realtime(engine)?;
    }

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
/// Velocity curve for MIDI input.
#[derive(Debug, Clone, Copy)]
enum VelocityCurve {
    /// Raw MIDI velocity mapped linearly (v / 127).
    Linear,
    /// Gentle boost to quiet notes. Good for stiff keys.
    Soft,
    /// Strong boost — light taps produce near-full volume.
    SuperSoft,
    /// Requires harder presses for volume. Good for overly sensitive controllers.
    Hard,
    /// All notes play at full velocity regardless of input.
    Full,
}

impl VelocityCurve {
    fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "linear" => Ok(VelocityCurve::Linear),
            "soft" => Ok(VelocityCurve::Soft),
            "supersoft" | "super-soft" | "super_soft" => Ok(VelocityCurve::SuperSoft),
            "hard" => Ok(VelocityCurve::Hard),
            "full" => Ok(VelocityCurve::Full),
            _ => Err(anyhow!(
                "Unknown velocity curve '{}'. Options: linear, soft, supersoft, hard, full", s
            )),
        }
    }

    /// Map raw MIDI velocity (1-127) to gain (0.0-1.0).
    fn apply(self, raw: u8) -> f64 {
        let v = raw as f64 / 127.0; // 0.0 to 1.0 linear
        match self {
            VelocityCurve::Linear => v,
            VelocityCurve::Soft => v.powf(0.5),         // square root — boosts quiet
            VelocityCurve::SuperSoft => v.powf(0.25),    // fourth root — strong boost
            VelocityCurve::Hard => v.powf(2.0),          // square — suppresses quiet
            VelocityCurve::Full => 1.0,                   // always max
        }
    }
}

/// A recorded event from piano mode.
enum RecordedEvent {
    Note {
        midi: i32,
        velocity: f64,
        /// Time in seconds since recording started.
        timestamp: f64,
        /// Duration in seconds (measured from note-on to note-off, or default).
        duration_secs: f64,
    },
    PedalDown {
        timestamp: f64,
    },
    PedalUp {
        timestamp: f64,
    },
}

fn cmd_piano(args: &[String]) -> Result<()> {
    use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
    use crossterm::terminal;
    use sound_cabinet::dsl::Command;

    if args.is_empty() {
        return Err(anyhow!(
            "Usage: sound-cabinet piano <score.sc> [instrument-name] [--midi [port]] [--velocity <curve>]"
        ));
    }

    // Parse args: score path, optional instrument name, optional flags
    let score_path = &args[0];
    let midi_flag = args.iter().any(|a| a == "--midi");
    let midi_port: Option<usize> = if midi_flag {
        args.iter()
            .position(|a| a == "--midi")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse().ok())
    } else {
        None
    };
    let velocity_curve = if let Some(curve_str) = parse_flag_str(args, "--velocity") {
        VelocityCurve::parse(curve_str)?
    } else {
        VelocityCurve::Linear
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

    // How long the DSP graph stays alive per note. Longer = notes can ring
    // longer (actual sound length is shaped by the instrument's decay).
    // 8 beats gives room for sustained notes without wasting too many resources.
    let note_duration_beats = 8.0;

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
        eprintln!("MIDI keyboard active — full range available. Velocity: {:?}", velocity_curve);
    }
    eprintln!();
    eprintln!("F1 = start/stop recording, F2 = save, F3 = discard, F4 = sustain pedal");
    eprintln!("Press keys to play. Esc or Ctrl+C to quit.");

    // Recording state
    let mut recording = false;
    let mut record_start = std::time::Instant::now();
    let mut recorded_notes: Vec<RecordedEvent> = Vec::new();
    let bpm = {
        let eng = engine.lock().unwrap();
        eng.bpm
    };

    // Track active MIDI notes for duration measurement: midi_note -> start time
    let mut active_notes: std::collections::HashMap<u8, std::time::Instant> = std::collections::HashMap::new();
    // Sustain pedal state
    let mut sustain_pedal = false;
    // Notes held by sustain pedal: will get note-off when pedal is released
    let mut sustained_notes: Vec<u8> = Vec::new();

    // Metronome: schedule a click on each beat while recording.
    let beat_interval = Duration::from_secs_f64(60.0 / bpm);
    let mut next_click = std::time::Instant::now();

    // Enter raw terminal mode
    terminal::enable_raw_mode()?;

    /// Build an expression for a note.
    fn build_note_expr(
        midi: i32,
        velocity: f64,
        inst_name: Option<&str>,
    ) -> Option<sound_cabinet::dsl::ast::Expr> {
        let freq = midi_to_hz(midi);

        if freq > 10000.0 || freq < 16.0 {
            let name = midi_to_name(midi);
            eprint!("  {} ({:.1} Hz) — out of range, skipped\r\n", name, freq);
            return None;
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

        let expr = if (velocity - 1.0).abs() > 0.01 {
            sound_cabinet::dsl::ast::Expr::Mul(
                Box::new(expr),
                Box::new(sound_cabinet::dsl::ast::Expr::Number(velocity)),
            )
        } else {
            expr
        };

        Some(expr)
    }

    let result = (|| -> Result<()> {
        loop {
            // Metronome: play a click on each beat while recording
            if recording && std::time::Instant::now() >= next_click {
                let mut eng = engine.lock().unwrap();
                eng.handle_command_relative(Command::PlayAt {
                    beat: 0.0,
                    expr: sound_cabinet::dsl::ast::Expr::Mul(
                        Box::new(sound_cabinet::dsl::ast::Expr::Pipe(
                            Box::new(sound_cabinet::dsl::ast::Expr::FnCall {
                                name: "sine".to_string(),
                                args: vec![sound_cabinet::dsl::ast::Expr::Number(1000.0)],
                            }),
                            Box::new(sound_cabinet::dsl::ast::Expr::FnCall {
                                name: "decay".to_string(),
                                args: vec![sound_cabinet::dsl::ast::Expr::Number(80.0)],
                            }),
                        )),
                        Box::new(sound_cabinet::dsl::ast::Expr::Number(0.3)),
                    ),
                    duration_beats: 0.25,
                    source: None,
                    voice_label: None,
                })
                .unwrap_or_else(|e| { let _ = e; });
                next_click += beat_interval;
            }

            // Check MIDI events (non-blocking)
            if let Some(ref rx) = midi_rx {
                while let Ok(midi_event) = rx.try_recv() {
                    match midi_event {
                        MidiEvent::NoteOn { note, velocity } => {
                            let vel = velocity_curve.apply(velocity);
                            active_notes.insert(note, std::time::Instant::now());

                            if let Some(expr) = build_note_expr(note as i32, vel, instrument_name) {
                                let name = midi_to_name(note as i32);
                                let freq = midi_to_hz(note as i32);

                                let mut eng = engine.lock().unwrap();
                                eng.play_live_note(expr, note as u16)
                                    .unwrap_or_else(|e| { let _ = e; });

                                let rec_marker = if recording { " REC" } else { "" };
                                eprint!("  {} ({:.1} Hz) vel {:.0}%{}\r\n",
                                    name, freq, vel * 100.0, rec_marker);

                                if recording {
                                    let timestamp = record_start.elapsed().as_secs_f64();
                                    recorded_notes.push(RecordedEvent::Note {
                                        midi: note as i32,
                                        velocity: vel,
                                        timestamp,
                                        duration_secs: 1.0, // default, updated on note-off
                                    });
                                }
                            }
                        }
                        MidiEvent::NoteOff { note } => {
                            if let Some(start_time) = active_notes.remove(&note) {
                                let held_secs = start_time.elapsed().as_secs_f64();
                                // Update recorded duration
                                if recording {
                                    for rec in recorded_notes.iter_mut().rev() {
                                        if let RecordedEvent::Note { midi, duration_secs, .. } = rec {
                                            if *midi == note as i32 {
                                                *duration_secs = held_secs;
                                                break;
                                            }
                                        }
                                    }
                                }
                                if sustain_pedal {
                                    // Don't release yet — pedal holds it
                                    sustained_notes.push(note);
                                } else {
                                    // Release the note immediately
                                    let mut eng = engine.lock().unwrap();
                                    eng.release_note(note as u16);
                                }
                            }
                        }
                        MidiEvent::SustainPedal(down) => {
                            sustain_pedal = down;
                            if !down {
                                let mut eng = engine.lock().unwrap();
                                for note in sustained_notes.drain(..) {
                                    eng.release_note(note as u16);
                                }
                            }
                            if recording {
                                let timestamp = record_start.elapsed().as_secs_f64();
                                if down {
                                    recorded_notes.push(RecordedEvent::PedalDown { timestamp });
                                } else {
                                    recorded_notes.push(RecordedEvent::PedalUp { timestamp });
                                }
                            }
                            let state = if down { "down" } else { "up" };
                            eprint!("  [sustain {}]\r\n", state);
                        }
                    }
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

                    match code {
                        // F1: toggle recording
                        KeyCode::F(1) => {
                            if recording {
                                recording = false;
                                eprint!("  -- Recording stopped ({} notes)\r\n", recorded_notes.len());
                            } else {
                                recording = true;
                                recorded_notes.clear();
                                record_start = std::time::Instant::now();
                                next_click = std::time::Instant::now() + beat_interval;
                                eprint!("  -- Recording started (bpm {})\r\n", bpm);
                            }
                        }

                        // F2: save recording to file
                        KeyCode::F(2) => {
                            if recorded_notes.is_empty() {
                                eprint!("  -- Nothing to save\r\n");
                            } else {
                                let filename = next_recording_filename();
                                match save_recording(
                                    &recorded_notes, bpm, instrument_name, score_path, &filename,
                                ) {
                                    Ok(()) => {
                                        eprint!("  -- Saved {} notes to {}\r\n",
                                            recorded_notes.len(), filename);
                                    }
                                    Err(e) => {
                                        eprint!("  -- Save failed: {}\r\n", e);
                                    }
                                }
                            }
                        }

                        // F3: discard recording
                        KeyCode::F(3) => {
                            let count = recorded_notes.len();
                            recorded_notes.clear();
                            recording = false;
                            eprint!("  -- Discarded {} notes\r\n", count);
                        }

                        // F4: toggle sustain pedal (keyboard substitute)
                        KeyCode::F(4) => {
                            sustain_pedal = !sustain_pedal;
                            if !sustain_pedal {
                                let mut eng = engine.lock().unwrap();
                                for note in sustained_notes.drain(..) {
                                    eng.release_note(note as u16);
                                }
                            }
                            if recording {
                                let timestamp = record_start.elapsed().as_secs_f64();
                                if sustain_pedal {
                                    recorded_notes.push(RecordedEvent::PedalDown { timestamp });
                                } else {
                                    recorded_notes.push(RecordedEvent::PedalUp { timestamp });
                                }
                            }
                            let state = if sustain_pedal { "down" } else { "up" };
                            eprint!("  [sustain {}]\r\n", state);
                        }

                        KeyCode::Char(c) => {
                            if let Some(midi) = key_to_midi(c) {
                                if let Some(expr) = build_note_expr(midi, 1.0, instrument_name) {
                                    let name = midi_to_name(midi);
                                    let freq = midi_to_hz(midi);

                                    let mut eng = engine.lock().unwrap();
                                    // Keyboard: use fixed duration since we don't get key-up
                                    eng.handle_command_relative(Command::PlayAt {
                                        beat: 0.0,
                                        expr,
                                        duration_beats: note_duration_beats,
                                        source: None,
                                        voice_label: None,
                                    })
                                    .unwrap_or_else(|e| { let _ = e; });

                                    let rec_marker = if recording { " REC" } else { "" };
                                    eprint!("  {} ({:.1} Hz){}\r\n", name, freq, rec_marker);

                                    if recording {
                                        let timestamp = record_start.elapsed().as_secs_f64();
                                        recorded_notes.push(RecordedEvent::Note {
                                            midi,
                                            velocity: 1.0,
                                            timestamp,
                                            duration_secs: note_duration_beats * 60.0 / bpm,
                                        });
                                    }
                                }
                            }
                        }

                        _ => {}
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

/// MIDI message types we care about.
#[derive(Debug)]
enum MidiEvent {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
    SustainPedal(bool), // true = down, false = up
}

/// Open a MIDI input connection. Returns a channel receiver for MIDI events,
/// the connection handle (must be kept alive), and the port name.
fn open_midi_input(
    port_index: Option<usize>,
) -> Result<(
    crossbeam_channel::Receiver<MidiEvent>,
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

    let (tx, rx) = crossbeam_channel::unbounded::<MidiEvent>();

    // MIDI callback: parse messages and send through channel
    let conn = midi_in
        .connect(
            port,
            "sound-cabinet-in",
            move |_timestamp, message, _data| {
                if message.len() >= 3 {
                    let status = message[0] & 0xF0;
                    let byte1 = message[1];
                    let byte2 = message[2];

                    match status {
                        0x90 if byte2 > 0 => {
                            let _ = tx.send(MidiEvent::NoteOn {
                                note: byte1,
                                velocity: byte2,
                            });
                        }
                        0x90 => {
                            // Note-on with velocity 0 = note-off per MIDI spec
                            let _ = tx.send(MidiEvent::NoteOff { note: byte1 });
                        }
                        0x80 => {
                            let _ = tx.send(MidiEvent::NoteOff { note: byte1 });
                        }
                        0xB0 if byte1 == 64 => {
                            // CC 64 = sustain pedal (>= 64 = on, < 64 = off)
                            let _ = tx.send(MidiEvent::SustainPedal(byte2 >= 64));
                        }
                        _ => {}
                    }
                }
            },
            (),
        )
        .map_err(|e| anyhow!("Cannot connect to MIDI port '{}': {e}", port_name))?;

    Ok((rx, conn, port_name))
}

/// Save recorded events as a .sc file with pattern + pedal events.
fn save_recording(
    events: &[RecordedEvent],
    bpm: f64,
    instrument_name: Option<&str>,
    score_path: &str,
    filename: &str,
) -> Result<()> {
    if events.is_empty() {
        return Err(anyhow!("No events to save"));
    }

    let voice_name = instrument_name.unwrap_or("sine");
    let beats_per_sec = bpm / 60.0;

    // Find total duration
    let mut max_secs = 0.0_f64;
    for ev in events {
        match ev {
            RecordedEvent::Note { timestamp, duration_secs, .. } => {
                max_secs = max_secs.max(timestamp + duration_secs);
            }
            RecordedEvent::PedalDown { timestamp } | RecordedEvent::PedalUp { timestamp } => {
                max_secs = max_secs.max(*timestamp);
            }
        }
    }
    let total_beats = (max_secs * beats_per_sec).ceil().max(1.0);

    let mut out = String::new();
    out.push_str(&format!("// Recorded in piano mode, bpm {}\n", bpm));
    out.push_str(&format!("import {}\n", score_path));
    out.push_str(&format!("bpm {}\n\n", bpm));

    // Pedal events go outside the pattern (they're score-level commands)
    let mut pedal_lines = Vec::new();
    // Pattern events
    let mut pattern_lines = Vec::new();

    for ev in events {
        match ev {
            RecordedEvent::Note { midi, velocity, timestamp, duration_secs } => {
                let beat = timestamp * beats_per_sec;
                let dur_beats = duration_secs * beats_per_sec;
                let note_name = midi_to_name(*midi);

                let beat_str = format_beat(beat);
                let dur_str = format_duration(dur_beats);

                if (*velocity - 1.0).abs() < 0.01 {
                    pattern_lines.push(format!(
                        "  at {} play {}({}) for {} beats  // {}",
                        beat_str, voice_name, note_name, dur_str, note_name
                    ));
                } else {
                    pattern_lines.push(format!(
                        "  at {} play {}({}) * {:.2} for {} beats  // {}",
                        beat_str, voice_name, note_name, velocity, dur_str, note_name
                    ));
                }
            }
            RecordedEvent::PedalDown { timestamp } => {
                let beat = timestamp * beats_per_sec;
                pedal_lines.push(format!("pedal down at {}", format_beat(beat)));
            }
            RecordedEvent::PedalUp { timestamp } => {
                let beat = timestamp * beats_per_sec;
                pedal_lines.push(format!("pedal up at {}", format_beat(beat)));
            }
        }
    }

    out.push_str(&format!("pattern recorded = {} beats\n", total_beats as i64));
    for line in &pattern_lines {
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');

    // Pedal events
    if !pedal_lines.is_empty() {
        out.push_str("// Sustain pedal events\n");
        for line in &pedal_lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }

    out.push_str("play recorded\n");

    std::fs::write(filename, &out)?;
    Ok(())
}

fn format_beat(beat: f64) -> String {
    if (beat - beat.round()).abs() < 0.01 {
        format!("{}", beat.round() as i64)
    } else {
        format!("{:.2}", beat)
    }
}

fn format_duration(dur_beats: f64) -> String {
    let rounded = (dur_beats * 4.0).round() / 4.0;
    let d = rounded.max(0.25);
    if (d - d.round()).abs() < 0.01 {
        format!("{}", d.round() as i64)
    } else {
        format!("{:.2}", d)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Find the next available filename like recorded_1.sc, recorded_2.sc, etc.
fn next_recording_filename() -> String {
    let mut n = 1;
    loop {
        let name = format!("recorded_{}.sc", n);
        if !std::path::Path::new(&name).exists() {
            return name;
        }
        n += 1;
    }
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
                        voice_label: None,
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
