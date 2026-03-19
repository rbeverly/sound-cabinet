use anyhow::{anyhow, Result};
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use super::ast::{
    Command, Expr, PatternEvent, RepeatBody, Script, SectionEntry, WeightedChoice,
};

#[derive(Parser)]
#[grammar = "dsl/grammar.pest"]
pub struct ScoreParser;

// ---------------------------------------------------------------------------
// Block grouping — first pass over raw lines
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Block {
    SingleLine(String),
    Pattern {
        header: String,
        body: Vec<String>,
    },
    Section {
        header: String,
        body: Vec<String>,
    },
    Repeat {
        header: String,
        body: Vec<String>,
    },
}

fn group_blocks(input: &str) -> Result<Vec<Block>> {
    let lines: Vec<&str> = input.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with("//") {
            i += 1;
            continue;
        }

        if trimmed.starts_with("pattern ") && trimmed.contains('=') {
            let header = trimmed.to_string();
            let mut body = Vec::new();
            i += 1;
            while i < lines.len() {
                let bline = lines[i];
                if bline.trim().is_empty() || bline.trim().starts_with("//") {
                    i += 1;
                    continue;
                }
                // Body lines must be indented
                if bline.starts_with(' ') || bline.starts_with('\t') {
                    body.push(bline.trim().to_string());
                    i += 1;
                } else {
                    break;
                }
            }
            blocks.push(Block::Pattern { header, body });
        } else if trimmed.starts_with("section ") && trimmed.contains('=') {
            let header = trimmed.to_string();
            let mut body = Vec::new();
            i += 1;
            while i < lines.len() {
                let bline = lines[i];
                if bline.trim().is_empty() || bline.trim().starts_with("//") {
                    i += 1;
                    continue;
                }
                if bline.starts_with(' ') || bline.starts_with('\t') {
                    body.push(bline.trim().to_string());
                    i += 1;
                } else {
                    break;
                }
            }
            blocks.push(Block::Section { header, body });
        } else if trimmed.starts_with("repeat ") && trimmed.ends_with('{') {
            let header = trimmed.to_string();
            let mut body = Vec::new();
            i += 1;
            while i < lines.len() {
                let bline = lines[i].trim();
                if bline == "}" {
                    i += 1;
                    break;
                }
                if !bline.is_empty() && !bline.starts_with("//") {
                    body.push(bline.to_string());
                }
                i += 1;
            }
            blocks.push(Block::Repeat { header, body });
        } else {
            blocks.push(Block::SingleLine(trimmed.to_string()));
            i += 1;
        }
    }

    Ok(blocks)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse an entire score file into a Script.
pub fn parse_script(input: &str) -> Result<Script> {
    let blocks = group_blocks(input)?;
    let mut commands = Vec::new();

    for block in blocks {
        match block {
            Block::SingleLine(line) => {
                if let Some(cmd) = parse_single_line(&line)? {
                    commands.push(cmd);
                }
            }
            Block::Pattern { header, body } => {
                commands.push(parse_pattern_def(&header, &body)?);
            }
            Block::Section { header, body } => {
                commands.push(parse_section_def(&header, &body)?);
            }
            Block::Repeat { header, body } => {
                commands.push(parse_repeat_block(&header, &body)?);
            }
        }
    }

    Ok(Script { commands })
}

/// Parse a single line of input into a Command (used by streaming mode).
pub fn parse_line(input: &str) -> Result<Command> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return Err(anyhow!("Empty or comment line"));
    }

    if let Ok(pairs) = ScoreParser::parse(Rule::voice_def, trimmed) {
        for pair in pairs {
            return parse_voice_def(pair);
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::fx_def, trimmed) {
        for pair in pairs {
            return parse_voice_def(pair);
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::instrument_def, trimmed) {
        for pair in pairs {
            return parse_voice_def(pair);
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::wave_def, trimmed) {
        for pair in pairs {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let samples: Vec<f64> = inner
                .filter(|p| p.as_rule() == Rule::number)
                .map(|p| p.as_str().parse().unwrap())
                .collect();
            return Ok(Command::WaveDef { name, samples });
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::pedal_down_stmt, trimmed) {
        for pair in pairs {
            let beat: f64 = pair.into_inner().next().unwrap().as_str().parse().unwrap();
            return Ok(Command::PedalDown { beat });
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::pedal_up_stmt, trimmed) {
        for pair in pairs {
            let beat: f64 = pair.into_inner().next().unwrap().as_str().parse().unwrap();
            return Ok(Command::PedalUp { beat });
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::bpm_stmt, trimmed) {
        for pair in pairs {
            return parse_bpm(pair);
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::at_stmt, trimmed) {
        for pair in pairs {
            return parse_at(pair);
        }
    }

    Err(anyhow!("Unrecognized line: {trimmed}"))
}

// ---------------------------------------------------------------------------
// Single-line parsing
// ---------------------------------------------------------------------------

fn parse_single_line(line: &str) -> Result<Option<Command>> {
    if line.is_empty() || line.starts_with("//") {
        return Ok(None);
    }

    // Try import
    if line.starts_with("import ") {
        let path = line.strip_prefix("import ").unwrap().trim().to_string();
        return Ok(Some(Command::Import { path }));
    }

    // Try play (top-level sequential) — must check before at_stmt since both
    // could start differently, but play_stmt is just "play <ident>"
    if line.starts_with("play ") && !line.contains(" for ") {
        if let Ok(pairs) = ScoreParser::parse(Rule::play_stmt, line) {
            for pair in pairs {
                let name = pair.into_inner().next().unwrap().as_str().to_string();
                return Ok(Some(Command::PlaySequential { name }));
            }
        }
    }

    // Try voice_def, fx_def, bpm_stmt, at_stmt via the full grammar
    if let Ok(pairs) = ScoreParser::parse(Rule::voice_def, line) {
        for pair in pairs {
            return Ok(Some(parse_voice_def(pair)?));
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::fx_def, line) {
        for pair in pairs {
            return Ok(Some(parse_voice_def(pair)?));
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::instrument_def, line) {
        for pair in pairs {
            return Ok(Some(parse_voice_def(pair)?));
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::wave_def, line) {
        for pair in pairs {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let samples: Vec<f64> = inner
                .filter(|p| p.as_rule() == Rule::number)
                .map(|p| p.as_str().parse().unwrap())
                .collect();
            return Ok(Some(Command::WaveDef { name, samples }));
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::pedal_down_stmt, line) {
        for pair in pairs {
            let beat: f64 = pair.into_inner().next().unwrap().as_str().parse().unwrap();
            return Ok(Some(Command::PedalDown { beat }));
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::pedal_up_stmt, line) {
        for pair in pairs {
            let beat: f64 = pair.into_inner().next().unwrap().as_str().parse().unwrap();
            return Ok(Some(Command::PedalUp { beat }));
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::bpm_stmt, line) {
        for pair in pairs {
            return Ok(Some(parse_bpm(pair)?));
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::at_stmt, line) {
        for pair in pairs {
            return Ok(Some(parse_at(pair)?));
        }
    }

    Err(anyhow!("Unrecognized line: {line}"))
}

// ---------------------------------------------------------------------------
// Block parsers
// ---------------------------------------------------------------------------

fn parse_pattern_def(header: &str, body: &[String]) -> Result<Command> {
    let pairs = ScoreParser::parse(Rule::pattern_header, header)
        .map_err(|e| anyhow!("Pattern header parse error:\n{e}"))?;

    let pair = pairs.into_iter().next().unwrap();
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let duration_beats: f64 = inner.next().unwrap().as_str().parse()?;

    let mut events = Vec::new();
    for line in body {
        let at_pairs = ScoreParser::parse(Rule::at_stmt, line)
            .map_err(|e| anyhow!("Pattern event parse error:\n{e}"))?;

        let at_pair = at_pairs.into_iter().next().unwrap();
        let mut at_inner = at_pair.into_inner();
        let beat_offset: f64 = at_inner.next().unwrap().as_str().parse()?;
        let expr = parse_expr(at_inner.next().unwrap())?;
        let duration: f64 = at_inner.next().unwrap().as_str().parse()?;

        events.push(PatternEvent {
            beat_offset,
            expr,
            duration_beats: duration,
        });
    }

    Ok(Command::PatternDef {
        name,
        duration_beats,
        events,
    })
}

fn parse_section_def(header: &str, body: &[String]) -> Result<Command> {
    let pairs = ScoreParser::parse(Rule::section_header, header)
        .map_err(|e| anyhow!("Section header parse error:\n{e}"))?;

    let pair = pairs.into_iter().next().unwrap();
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let duration_beats: f64 = inner.next().unwrap().as_str().parse()?;

    let mut entries = Vec::new();
    for line in body {
        if line.starts_with("repeat ") {
            let entry_pairs = ScoreParser::parse(Rule::section_entry_repeat, line)
                .map_err(|e| anyhow!("Section repeat entry parse error:\n{e}"))?;
            let entry_pair = entry_pairs.into_iter().next().unwrap();
            let mut ei = entry_pair.into_inner();
            let entry_name = ei.next().unwrap().as_str().to_string();
            let every_beats: f64 = ei.next().unwrap().as_str().parse()?;
            entries.push(SectionEntry::RepeatEvery {
                name: entry_name,
                every_beats,
            });
        } else if line.starts_with("play ") {
            let entry_pairs = ScoreParser::parse(Rule::section_entry_play, line)
                .map_err(|e| anyhow!("Section play entry parse error:\n{e}"))?;
            let entry_pair = entry_pairs.into_iter().next().unwrap();
            let entry_name = entry_pair
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            entries.push(SectionEntry::Play { name: entry_name });
        } else {
            return Err(anyhow!("Unrecognized section entry: {line}"));
        }
    }

    Ok(Command::SectionDef {
        name,
        duration_beats,
        entries,
    })
}

fn parse_repeat_block(header: &str, body: &[String]) -> Result<Command> {
    let pairs = ScoreParser::parse(Rule::repeat_header, header)
        .map_err(|e| anyhow!("Repeat header parse error:\n{e}"))?;

    let pair = pairs.into_iter().next().unwrap();
    let mut inner = pair.into_inner();
    let count: u32 = inner.next().unwrap().as_str().parse()?;

    let mut items = Vec::new();
    for line in body {
        if line.starts_with("pick ") {
            let pick_pairs = ScoreParser::parse(Rule::pick_stmt, line)
                .map_err(|e| anyhow!("Pick parse error:\n{e}"))?;
            let pick_pair = pick_pairs.into_iter().next().unwrap();
            let mut choices = Vec::new();
            for wi in pick_pair.into_inner() {
                if wi.as_rule() == Rule::weighted_item {
                    let mut wi_inner = wi.into_inner();
                    let name = wi_inner.next().unwrap().as_str().to_string();
                    let weight = wi_inner
                        .next()
                        .map(|n| n.as_str().parse::<f64>().unwrap_or(1.0))
                        .unwrap_or(1.0);
                    choices.push(WeightedChoice { name, weight });
                }
            }
            items.push(RepeatBody::Pick(choices));
        } else if line.starts_with("shuffle ") {
            let shuf_pairs = ScoreParser::parse(Rule::shuffle_stmt, line)
                .map_err(|e| anyhow!("Shuffle parse error:\n{e}"))?;
            let shuf_pair = shuf_pairs.into_iter().next().unwrap();
            let names: Vec<String> = shuf_pair
                .into_inner()
                .filter(|p| p.as_rule() == Rule::ident)
                .map(|p| p.as_str().to_string())
                .collect();
            items.push(RepeatBody::Shuffle(names));
        } else if line.starts_with("play ") {
            if let Ok(play_pairs) = ScoreParser::parse(Rule::play_stmt, line) {
                let play_pair = play_pairs.into_iter().next().unwrap();
                let name = play_pair
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str()
                    .to_string();
                items.push(RepeatBody::Play(name));
            } else {
                return Err(anyhow!("Unrecognized repeat body line: {line}"));
            }
        } else {
            return Err(anyhow!("Unrecognized repeat body line: {line}"));
        }
    }

    Ok(Command::RepeatBlock { count, body: items })
}

// ---------------------------------------------------------------------------
// Expression / command parsers (shared)
// ---------------------------------------------------------------------------

fn parse_voice_def(pair: Pair<Rule>) -> Result<Command> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let expr = parse_expr(inner.next().unwrap())?;
    Ok(Command::VoiceDef { name, expr })
}

fn parse_bpm(pair: Pair<Rule>) -> Result<Command> {
    let mut inner = pair.into_inner();
    let bpm: f64 = inner.next().unwrap().as_str().parse()?;
    Ok(Command::SetBpm(bpm))
}

fn parse_at(pair: Pair<Rule>) -> Result<Command> {
    let mut inner = pair.into_inner();
    let beat: f64 = inner.next().unwrap().as_str().parse()?;
    let expr = parse_expr(inner.next().unwrap())?;
    let duration_beats: f64 = inner.next().unwrap().as_str().parse()?;
    Ok(Command::PlayAt {
        beat,
        expr,
        duration_beats,
    })
}

fn parse_expr(pair: Pair<Rule>) -> Result<Expr> {
    match pair.as_rule() {
        Rule::expr | Rule::pipe_expr | Rule::sum_expr | Rule::mul_expr => {
            parse_binary_expr(pair)
        }
        Rule::atom => parse_atom(pair),
        Rule::fn_call => parse_fn_call(pair),
        Rule::range => {
            let mut inner = pair.into_inner();
            let start: f64 = inner.next().unwrap().as_str().parse()?;
            let end: f64 = inner.next().unwrap().as_str().parse()?;
            Ok(Expr::Range(start, end))
        }
        Rule::number => Ok(Expr::Number(pair.as_str().parse()?)),
        Rule::note_name => Ok(Expr::Number(note_name_to_hz(pair.as_str())?)),
        Rule::ident => Ok(Expr::VoiceRef(pair.as_str().to_string())),
        _ => Err(anyhow!("Unexpected rule: {:?}", pair.as_rule())),
    }
}

/// Convert a note name like "A4", "Bb3", "C#5", "Fs4" to frequency in Hz.
/// Uses standard tuning: A4 = 440 Hz.
fn note_name_to_hz(s: &str) -> Result<f64> {
    let mut chars = s.chars();

    let letter = chars.next().ok_or_else(|| anyhow!("Empty note name"))?;
    let semitone_base: i32 = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return Err(anyhow!("Invalid note letter: {letter}")),
    };

    // Peek at next char: could be accidental or octave digit
    let rest: String = chars.collect();
    let (accidental, octave_str) = if rest.starts_with('#') || rest.starts_with('s') {
        (1i32, &rest[1..])
    } else if rest.starts_with('b') {
        (-1i32, &rest[1..])
    } else {
        (0i32, rest.as_str())
    };

    let octave: i32 = octave_str
        .parse()
        .map_err(|_| anyhow!("Invalid octave in note: {s}"))?;

    let midi = (octave + 1) * 12 + semitone_base + accidental;
    let hz = 440.0 * 2.0_f64.powf((midi as f64 - 69.0) / 12.0);
    Ok(hz)
}

/// Resolve a chord name like "Cm7", "Fmaj7", "Am9" to a vector of frequencies in Hz.
/// Returns None if the string isn't a valid chord name.
///
/// Format: Root[Accidental]Quality[Octave]
///   Root: A-G
///   Accidental: # s b (optional)
///   Quality: maj, m, min, dim, aug, 7, dom7, m7, min7, maj7, dim7, aug7,
///            9, dom9, m9, min9, maj9, sus2, sus4 (or empty = major triad)
///   Octave: 0-9 (optional, defaults to 4)
pub fn resolve_chord(name: &str) -> Option<Vec<f64>> {
    let mut chars = name.chars().peekable();

    // Parse root letter
    let letter = chars.next()?;
    let semitone_base: i32 = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };

    // Parse optional accidental
    let rest: String = chars.collect();
    let (accidental, rest) = if rest.starts_with('#') || rest.starts_with('s') {
        (1i32, &rest[1..])
    } else if rest.starts_with('b') {
        (-1i32, &rest[1..])
    } else {
        (0i32, rest.as_str())
    };

    // If nothing left after root+accidental, it's a bare note letter, not a chord
    if rest.is_empty() {
        return None;
    }

    // Try to match quality suffix (longest first to avoid partial matches)
    // Each entry: (suffix, intervals)
    let qualities: &[(&str, &[i32])] = &[
        ("maj9", &[0, 4, 7, 11, 14]),
        ("min9", &[0, 3, 7, 10, 14]),
        ("maj7", &[0, 4, 7, 11]),
        ("min7", &[0, 3, 7, 10]),
        ("dim7", &[0, 3, 6, 9]),
        ("aug7", &[0, 4, 8, 10]),
        ("dom9", &[0, 4, 7, 10, 14]),
        ("dom7", &[0, 4, 7, 10]),
        ("sus2", &[0, 2, 7]),
        ("sus4", &[0, 5, 7]),
        ("min", &[0, 3, 7]),
        ("maj", &[0, 4, 7]),
        ("dim", &[0, 3, 6]),
        ("aug", &[0, 4, 8]),
        ("m9", &[0, 3, 7, 10, 14]),
        ("m7", &[0, 3, 7, 10]),
        ("m", &[0, 3, 7]),
        ("9", &[0, 4, 7, 10, 14]),
        ("7", &[0, 4, 7, 10]),
    ];

    let (intervals, after_quality) = qualities
        .iter()
        .find_map(|(suffix, intervals)| {
            rest.strip_prefix(suffix).map(|remaining| (*intervals, remaining))
        })?;

    // Parse optional octave digit, default to 4
    let octave: i32 = if after_quality.is_empty() {
        4
    } else if after_quality.len() == 1 && after_quality.as_bytes()[0].is_ascii_digit() {
        (after_quality.as_bytes()[0] - b'0') as i32
    } else {
        return None; // leftover unparsed chars
    };

    // Convert each interval to Hz
    let root_midi = (octave + 1) * 12 + semitone_base + accidental;
    let notes: Vec<f64> = intervals
        .iter()
        .map(|interval| {
            let midi = root_midi + interval;
            440.0 * 2.0_f64.powf((midi as f64 - 69.0) / 12.0)
        })
        .collect();

    Some(notes)
}

fn parse_binary_expr(pair: Pair<Rule>) -> Result<Expr> {
    let rule = pair.as_rule();
    let mut inner = pair.into_inner();

    let first = inner.next().unwrap();
    let mut left = parse_expr(first)?;

    while let Some(next) = inner.next() {
        match rule {
            Rule::mul_expr => {
                // mul_expr has interleaved mul_op and atom tokens
                if next.as_rule() == Rule::mul_op {
                    let op = next.as_str();
                    let operand = inner.next().unwrap();
                    let right = parse_expr(operand)?;
                    left = if op == "/" {
                        Expr::Div(Box::new(left), Box::new(right))
                    } else {
                        Expr::Mul(Box::new(left), Box::new(right))
                    };
                } else {
                    let right = parse_expr(next)?;
                    left = Expr::Mul(Box::new(left), Box::new(right));
                }
            }
            Rule::sum_expr => {
                if next.as_rule() == Rule::sum_op {
                    let op = next.as_str();
                    let operand = inner.next().unwrap();
                    let right = parse_expr(operand)?;
                    left = if op == "-" {
                        Expr::Sub(Box::new(left), Box::new(right))
                    } else {
                        Expr::Sum(Box::new(left), Box::new(right))
                    };
                } else {
                    let right = parse_expr(next)?;
                    left = Expr::Sum(Box::new(left), Box::new(right));
                }
            }
            _ => {
                let right = parse_expr(next)?;
                left = match rule {
                    Rule::pipe_expr | Rule::expr => Expr::Pipe(Box::new(left), Box::new(right)),
                    _ => unreachable!(),
                };
            }
        }
    }

    Ok(left)
}

fn parse_atom(pair: Pair<Rule>) -> Result<Expr> {
    let inner = pair.into_inner().next().unwrap();
    parse_expr(inner)
}

fn parse_fn_call(pair: Pair<Rule>) -> Result<Expr> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    let mut args = Vec::new();
    if let Some(arg_list) = inner.next() {
        for arg_pair in arg_list.into_inner() {
            args.push(parse_expr(arg_pair)?);
        }
    }

    Ok(Expr::FnCall { name, args })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bpm() {
        let script = parse_script("bpm 120\n").unwrap();
        match &script.commands[0] {
            Command::SetBpm(bpm) => assert_eq!(*bpm, 120.0),
            _ => panic!("Expected SetBpm"),
        }
    }

    #[test]
    fn test_parse_voice_def() {
        let script = parse_script("voice pad = sine(440)\n").unwrap();
        match &script.commands[0] {
            Command::VoiceDef { name, .. } => assert_eq!(name, "pad"),
            _ => panic!("Expected VoiceDef"),
        }
    }

    #[test]
    fn test_parse_at() {
        let script = parse_script("at 0 play sine(440) for 2 beats\n").unwrap();
        match &script.commands[0] {
            Command::PlayAt {
                beat,
                duration_beats,
                ..
            } => {
                assert_eq!(*beat, 0.0);
                assert_eq!(*duration_beats, 2.0);
            }
            _ => panic!("Expected PlayAt"),
        }
    }

    #[test]
    fn test_parse_complex_expr() {
        let script =
            parse_script("voice x = (saw(40) + 0.5 * sine(80)) >> lowpass(2000, 0.7)\n").unwrap();
        match &script.commands[0] {
            Command::VoiceDef { name, expr } => {
                assert_eq!(name, "x");
                match expr {
                    Expr::Pipe(_, _) => {}
                    other => panic!("Expected Pipe, got {other:?}"),
                }
            }
            _ => panic!("Expected VoiceDef"),
        }
    }

    #[test]
    fn test_parse_script() {
        let input = "bpm 120\nat 0 play sine(440) for 2 beats\n";
        let script = parse_script(input).unwrap();
        assert_eq!(script.commands.len(), 2);
    }

    #[test]
    fn test_parse_import() {
        let script = parse_script("import voices/kick.sc\n").unwrap();
        match &script.commands[0] {
            Command::Import { path } => assert_eq!(path, "voices/kick.sc"),
            _ => panic!("Expected Import"),
        }
    }

    #[test]
    fn test_parse_pattern() {
        let input = "\
pattern drums = 4 beats
  at 0 play kick for 0.5 beats
  at 1 play snare for 0.25 beats
  at 2 play kick for 0.5 beats
  at 3 play snare for 0.25 beats
";
        let script = parse_script(input).unwrap();
        match &script.commands[0] {
            Command::PatternDef {
                name,
                duration_beats,
                events,
            } => {
                assert_eq!(name, "drums");
                assert_eq!(*duration_beats, 4.0);
                assert_eq!(events.len(), 4);
                assert_eq!(events[0].beat_offset, 0.0);
                assert_eq!(events[1].beat_offset, 1.0);
            }
            _ => panic!("Expected PatternDef"),
        }
    }

    #[test]
    fn test_parse_section() {
        let input = "\
section verse = 16 beats
  repeat drums every 4 beats
  play chords
";
        let script = parse_script(input).unwrap();
        match &script.commands[0] {
            Command::SectionDef {
                name,
                duration_beats,
                entries,
            } => {
                assert_eq!(name, "verse");
                assert_eq!(*duration_beats, 16.0);
                assert_eq!(entries.len(), 2);
            }
            _ => panic!("Expected SectionDef"),
        }
    }

    #[test]
    fn test_parse_repeat_block() {
        let input = "\
repeat 4 {
  pick [verse:3, chorus:1]
  play bridge
}
";
        let script = parse_script(input).unwrap();
        match &script.commands[0] {
            Command::RepeatBlock { count, body } => {
                assert_eq!(*count, 4);
                assert_eq!(body.len(), 2);
                match &body[0] {
                    RepeatBody::Pick(choices) => {
                        assert_eq!(choices.len(), 2);
                        assert_eq!(choices[0].name, "verse");
                        assert_eq!(choices[0].weight, 3.0);
                        assert_eq!(choices[1].name, "chorus");
                        assert_eq!(choices[1].weight, 1.0);
                    }
                    _ => panic!("Expected Pick"),
                }
                match &body[1] {
                    RepeatBody::Play(name) => assert_eq!(name, "bridge"),
                    _ => panic!("Expected Play"),
                }
            }
            _ => panic!("Expected RepeatBlock"),
        }
    }

    #[test]
    fn test_parse_play_sequential() {
        let script = parse_script("play intro\n").unwrap();
        match &script.commands[0] {
            Command::PlaySequential { name } => assert_eq!(name, "intro"),
            _ => panic!("Expected PlaySequential"),
        }
    }

    #[test]
    fn test_note_name_a4() {
        let script = parse_script("voice x = sine(A4)\n").unwrap();
        match &script.commands[0] {
            Command::VoiceDef { expr, .. } => match expr {
                Expr::FnCall { args, .. } => match &args[0] {
                    Expr::Number(hz) => assert!((hz - 440.0).abs() < 0.01),
                    other => panic!("Expected Number, got {other:?}"),
                },
                other => panic!("Expected FnCall, got {other:?}"),
            },
            other => panic!("Expected VoiceDef, got {other:?}"),
        }
    }

    #[test]
    fn test_note_name_c4() {
        let hz = note_name_to_hz("C4").unwrap();
        assert!((hz - 261.63).abs() < 0.01, "C4 should be ~261.63, got {hz}");
    }

    #[test]
    fn test_note_name_bb3() {
        let hz = note_name_to_hz("Bb3").unwrap();
        assert!(
            (hz - 233.08).abs() < 0.01,
            "Bb3 should be ~233.08, got {hz}"
        );
    }

    #[test]
    fn test_note_name_fsharp4() {
        let hz_sharp = note_name_to_hz("F#4").unwrap();
        let hz_s = note_name_to_hz("Fs4").unwrap();
        assert!(
            (hz_sharp - 369.99).abs() < 0.01,
            "F#4 should be ~369.99, got {hz_sharp}"
        );
        assert!(
            (hz_sharp - hz_s).abs() < 0.001,
            "F#4 and Fs4 should be equal"
        );
    }

    #[test]
    fn test_parse_shuffle() {
        let input = "\
repeat 3 {
  shuffle [a, b, c]
}
";
        let script = parse_script(input).unwrap();
        match &script.commands[0] {
            Command::RepeatBlock { count, body } => {
                assert_eq!(*count, 3);
                match &body[0] {
                    RepeatBody::Shuffle(names) => {
                        assert_eq!(names, &["a", "b", "c"]);
                    }
                    _ => panic!("Expected Shuffle"),
                }
            }
            _ => panic!("Expected RepeatBlock"),
        }
    }

    #[test]
    fn test_resolve_chord_cm7() {
        let notes = resolve_chord("Cm7").unwrap();
        assert_eq!(notes.len(), 4);
        let c4 = note_name_to_hz("C4").unwrap();
        let eb4 = note_name_to_hz("Eb4").unwrap();
        let g4 = note_name_to_hz("G4").unwrap();
        let bb4 = note_name_to_hz("Bb4").unwrap();
        assert!((notes[0] - c4).abs() < 0.01, "Root should be C4");
        assert!((notes[1] - eb4).abs() < 0.01, "3rd should be Eb4");
        assert!((notes[2] - g4).abs() < 0.01, "5th should be G4");
        assert!((notes[3] - bb4).abs() < 0.01, "7th should be Bb4");
    }

    #[test]
    fn test_resolve_chord_fmaj7() {
        let notes = resolve_chord("Fmaj7").unwrap();
        assert_eq!(notes.len(), 4);
        let f4 = note_name_to_hz("F4").unwrap();
        let a4 = note_name_to_hz("A4").unwrap();
        assert!((notes[0] - f4).abs() < 0.01);
        assert!((notes[1] - a4).abs() < 0.01);
    }

    #[test]
    fn test_resolve_chord_am() {
        let notes = resolve_chord("Am").unwrap();
        assert_eq!(notes.len(), 3); // minor triad
        let a4 = note_name_to_hz("A4").unwrap();
        assert!((notes[0] - a4).abs() < 0.01);
    }

    #[test]
    fn test_resolve_chord_gdom7() {
        let notes = resolve_chord("Gdom7").unwrap();
        assert_eq!(notes.len(), 4);
        let g4 = note_name_to_hz("G4").unwrap();
        let b4 = note_name_to_hz("B4").unwrap();
        assert!((notes[0] - g4).abs() < 0.01);
        assert!((notes[1] - b4).abs() < 0.01);
    }

    #[test]
    fn test_resolve_chord_bbm7() {
        let notes = resolve_chord("Bbm7").unwrap();
        assert_eq!(notes.len(), 4);
        let bb4 = note_name_to_hz("Bb4").unwrap();
        assert!((notes[0] - bb4).abs() < 0.01);
    }

    #[test]
    fn test_resolve_chord_with_octave() {
        let notes3 = resolve_chord("Cm73").unwrap();
        let notes4 = resolve_chord("Cm7").unwrap();
        // Octave 3 should be one octave below octave 4
        assert!((notes3[0] - notes4[0] / 2.0).abs() < 0.01);
    }

    #[test]
    fn test_resolve_chord_invalid() {
        assert!(resolve_chord("foo").is_none());
        assert!(resolve_chord("X7").is_none());
        assert!(resolve_chord("C").is_none()); // bare letter, not a chord
    }
}
