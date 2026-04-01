use anyhow::{anyhow, Result};
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use std::collections::HashMap;

use super::ast::{
    Command, DefKind, Expr, PatternEvent, PatternRef, RepeatBody, Script, SectionEntry,
    WeightedChoice, WithMap,
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
        body: Vec<SectionBodyItem>,
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
        } else if trimmed.starts_with("section ") {
            let header = trimmed.to_string();
            let mut body: Vec<SectionBodyItem> = Vec::new();
            i += 1;
            while i < lines.len() {
                let bline = lines[i];
                let btrimmed = bline.trim();
                if btrimmed.is_empty() || btrimmed.starts_with("//") {
                    i += 1;
                    continue;
                }
                if bline.starts_with(' ') || bline.starts_with('\t') {
                    // Check for nested repeat block: "repeat N {"
                    if btrimmed.starts_with("repeat ") && btrimmed.ends_with('{') {
                        let rh = btrimmed.to_string();
                        let mut rb = Vec::new();
                        i += 1;
                        while i < lines.len() {
                            let rline = lines[i].trim();
                            if rline == "}" {
                                i += 1;
                                break;
                            }
                            if !rline.is_empty() && !rline.starts_with("//") {
                                rb.push(rline.to_string());
                            }
                            i += 1;
                        }
                        body.push(SectionBodyItem::RepeatBlock { header: rh, body: rb });
                    } else {
                        body.push(SectionBodyItem::Line(btrimmed.to_string()));
                        i += 1;
                    }
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

/// Try to parse a single line as any known command type.
/// Returns Ok(Some(cmd)) if parsed, Ok(None) if empty/comment, Err if unrecognized.
/// This is the single source of truth for line-level parsing — both streaming mode
/// and script mode call this function.
fn try_parse_command(line: &str) -> Result<Option<Command>> {
    if line.is_empty() || line.starts_with("//") {
        return Ok(None);
    }

    // Import (string prefix check, not grammar rule)
    if line.starts_with("import ") {
        let path = line.strip_prefix("import ").unwrap().trim().to_string();
        return Ok(Some(Command::Import { path }));
    }

    // Top-level sequential play: "play intro" (but not "play X for N beats")
    if line.starts_with("play ") && !line.contains(" for ") {
        if let Ok(pairs) = ScoreParser::parse(Rule::play_stmt, line) {
            for pair in pairs {
                let pref = parse_pattern_ref(pair.into_inner().next().unwrap())?;
                return Ok(Some(Command::PlaySequential { pattern: pref }));
            }
        }
    }

    // Voice / fx / instrument definitions
    for (rule, kind) in [
        (Rule::voice_def, DefKind::Voice),
        (Rule::fx_def, DefKind::Fx),
        (Rule::instrument_def, DefKind::Instrument),
    ] {
        if let Ok(pairs) = ScoreParser::parse(rule, line) {
            for pair in pairs {
                return Ok(Some(parse_voice_def(pair, kind)?));
            }
        }
    }

    // Wavetable definition
    if let Ok(pairs) = ScoreParser::parse(Rule::wave_def, line) {
        for pair in pairs {
            return Ok(Some(parse_wave_def_pair(pair)?));
        }
    }

    // Normalize
    if let Ok(pairs) = ScoreParser::parse(Rule::normalize_stmt, line) {
        for pair in pairs {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let target: f64 = inner.next().unwrap().as_str().parse()
                .map_err(|_| anyhow!("normalize: invalid target level"))?;
            return Ok(Some(Command::Normalize { name, target }));
        }
    }

    // Pedal events
    if let Ok(pairs) = ScoreParser::parse(Rule::pedal_down_stmt, line) {
        for pair in pairs {
            let (voices, beat) = parse_pedal_inner(pair)?;
            return Ok(Some(Command::PedalDown { beat, voices }));
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::pedal_up_stmt, line) {
        for pair in pairs {
            let (voices, beat) = parse_pedal_inner(pair)?;
            return Ok(Some(Command::PedalUp { beat, voices }));
        }
    }

    // BPM
    if let Ok(pairs) = ScoreParser::parse(Rule::bpm_stmt, line) {
        for pair in pairs {
            return Ok(Some(parse_bpm(pair)?));
        }
    }

    // Swing / humanize
    if let Ok(pairs) = ScoreParser::parse(Rule::master_stmt, line) {
        for pair in pairs {
            let inner = pair.into_inner().next()
                .ok_or_else(|| anyhow!("Expected master sub-command"))?;
            match inner.as_rule() {
                Rule::master_compress => {
                    let vals: Vec<f64> = inner.into_inner()
                        .map(|p| p.as_str().parse::<f64>())
                        .collect::<std::result::Result<Vec<_>, _>>()
                        .map_err(|_| anyhow!("Invalid number in master compress"))?;
                    if vals.is_empty() {
                        return Err(anyhow!("master compress requires at least one argument"));
                    }
                    return Ok(Some(Command::MasterCompress(vals)));
                }
                Rule::master_ceiling => {
                    let val: f64 = inner.into_inner().next()
                        .ok_or_else(|| anyhow!("Expected value in master ceiling"))?
                        .as_str().parse()
                        .map_err(|_| anyhow!("Invalid number in master ceiling"))?;
                    return Ok(Some(Command::MasterCeiling(val)));
                }
                Rule::master_gain => {
                    let val: f64 = inner.into_inner().next()
                        .ok_or_else(|| anyhow!("Expected value in master gain"))?
                        .as_str().parse()
                        .map_err(|_| anyhow!("Invalid number in master gain"))?;
                    return Ok(Some(Command::MasterGain(val)));
                }
                _ => {}
            }
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::swing_stmt, line) {
        for pair in pairs {
            let val: f64 = pair.into_inner().next()
                .ok_or_else(|| anyhow!("Expected value in swing statement"))?
                .as_str().parse()
                .map_err(|_| anyhow!("Invalid number in swing statement"))?;
            return Ok(Some(Command::SetSwing(val)));
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::humanize_stmt, line) {
        for pair in pairs {
            let val: f64 = pair.into_inner().next()
                .ok_or_else(|| anyhow!("Expected value in humanize statement"))?
                .as_str().parse()
                .map_err(|_| anyhow!("Invalid number in humanize statement"))?;
            return Ok(Some(Command::SetHumanize(val)));
        }
    }
    if let Ok(pairs) = ScoreParser::parse(Rule::with_stmt, line) {
        for pair in pairs {
            let map = parse_mappings(pair.into_inner());
            return Ok(Some(Command::SetWith(map)));
        }
    }

    // Scheduled event: at N play expr for M beats
    if let Ok(pairs) = ScoreParser::parse(Rule::at_stmt, line) {
        for pair in pairs {
            return Ok(Some(parse_at(pair)?));
        }
    }

    Err(anyhow!("Unrecognized line: {line}"))
}

/// Parse a pedal statement's inner pairs: optional voice list + beat number.
fn parse_pedal_inner(pair: Pair<Rule>) -> Result<(Vec<String>, f64)> {
    let mut voices = Vec::new();
    let mut beat = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::pedal_voices => {
                for v in inner.into_inner() {
                    if v.as_rule() == Rule::ident {
                        voices.push(v.as_str().to_string());
                    }
                }
            }
            Rule::number => {
                beat = Some(inner.as_str().parse::<f64>()
                    .map_err(|_| anyhow!("Invalid number in pedal statement"))?);
            }
            _ => {}
        }
    }

    let beat = beat.ok_or_else(|| anyhow!("Expected beat position in pedal statement"))?;
    Ok((voices, beat))
}

/// Extract the outermost voice/instrument name from an expression.
/// Used to populate `voice_label` on PlayAt commands.
pub fn extract_voice_label(expr: &Expr) -> Option<String> {
    match expr {
        Expr::FnCall { name, .. } => Some(name.clone()),
        Expr::VoiceRef(name) => Some(name.clone()),
        Expr::Pipe(left, _) => extract_voice_label(left),
        Expr::Mul(left, right) => {
            // Skip the gain side (Number), extract from the voice side
            match (left.as_ref(), right.as_ref()) {
                (Expr::Number(_), other) | (other, Expr::Number(_)) => extract_voice_label(other),
                _ => extract_voice_label(left).or_else(|| extract_voice_label(right)),
            }
        }
        Expr::Sum(left, _) => extract_voice_label(left),
        _ => None,
    }
}

/// Extract a wave_def pair into a WaveDef command.
fn parse_wave_def_pair(pair: Pair<Rule>) -> Result<Command> {
    let mut inner = pair.into_inner();
    let name = inner.next()
        .ok_or_else(|| anyhow!("Expected name in wave definition"))?
        .as_str().to_string();
    let samples: Result<Vec<f64>> = inner
        .filter(|p| p.as_rule() == Rule::number)
        .map(|p| p.as_str().parse::<f64>().map_err(|e| anyhow!("Invalid number in wave definition: {e}")))
        .collect();
    Ok(Command::WaveDef { name, samples: samples? })
}

/// Parse a single line of input into a Command (used by streaming mode).
pub fn parse_line(input: &str) -> Result<Command> {
    let trimmed = input.trim();
    match try_parse_command(trimmed)? {
        Some(cmd) => Ok(cmd),
        None => Err(anyhow!("Empty or comment line")),
    }
}

// ---------------------------------------------------------------------------
// Single-line parsing (used by script mode) — delegates to try_parse_command
// ---------------------------------------------------------------------------

fn parse_single_line(line: &str) -> Result<Option<Command>> {
    try_parse_command(line)
}

// ---------------------------------------------------------------------------
// Block parsers
// ---------------------------------------------------------------------------

/// Parse a sequence of `mapping` pairs into a WithMap.
fn parse_mappings(pairs: pest::iterators::Pairs<Rule>) -> WithMap {
    let mut map = HashMap::new();
    for pair in pairs {
        if pair.as_rule() == Rule::mapping {
            let mut inner = pair.into_inner();
            let from = inner.next().unwrap().as_str().to_string();
            let to = inner.next().unwrap().as_str().to_string();
            map.insert(from, to);
        }
    }
    map
}

/// Parse an optional with_clause from a pair's inner iterator.
/// Returns Some(WithMap) if a with_clause is present, None otherwise.
fn parse_optional_with(inner: &mut pest::iterators::Pairs<Rule>) -> Option<WithMap> {
    // Peek at remaining pairs for a with_clause
    for pair in inner {
        if pair.as_rule() == Rule::with_clause {
            return Some(parse_mappings(pair.into_inner()));
        }
    }
    None
}

fn parse_pattern_def(header: &str, body: &[String]) -> Result<Command> {
    let pairs = ScoreParser::parse(Rule::pattern_header, header)
        .map_err(|e| anyhow!("Pattern header parse error:\n{e}"))?;

    let pair = pairs.into_iter().next().unwrap();
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let duration_beats: f64 = inner.next().unwrap().as_str().parse()?;
    // skip beat_unit
    let _ = inner.next(); // "beats" / "beat"

    // Parse optional modifiers: swing 0.65, humanize 5
    let mut swing = None;
    let mut humanize = None;
    for modifier in inner {
        if modifier.as_rule() == Rule::pattern_modifier {
            let text = modifier.as_str().trim();
            if text.starts_with("swing") {
                let val: f64 = text.strip_prefix("swing").unwrap().trim().parse()?;
                swing = Some(val);
            } else if text.starts_with("humanize") {
                let val: f64 = text.strip_prefix("humanize").unwrap().trim().parse()?;
                humanize = Some(val);
            }
        }
    }

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
        swing,
        humanize,
    })
}

fn parse_section_def(header: &str, body: &[SectionBodyItem]) -> Result<Command> {
    let pairs = ScoreParser::parse(Rule::section_header, header)
        .map_err(|e| anyhow!("Section header parse error:\n{e}"))?;

    let pair = pairs.into_iter().next().unwrap();
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    // Duration is optional: check if next token is a number (duration) or with_clause
    let mut duration_beats: Option<f64> = None;
    let mut section_with: Option<WithMap> = None;

    for token in inner {
        match token.as_rule() {
            Rule::number => {
                duration_beats = Some(token.as_str().parse()?);
            }
            Rule::beat_unit => {} // skip
            Rule::with_clause => {
                section_with = parse_with_clause(token);
            }
            _ => {}
        }
    }

    let mut entries = Vec::new();
    for item in body {
        match item {
            SectionBodyItem::Line(line) => {
                let entry = parse_section_entry(line)?;
                entries.push(entry);
            }
            SectionBodyItem::RepeatBlock { header: rh, body: rb } => {
                let repeat_cmd = parse_repeat_block(rh, rb)?;
                if let Command::RepeatBlock { count, body: repeat_body } = repeat_cmd {
                    entries.push(SectionEntry::RepeatBlock { count, body: repeat_body });
                }
            }
        }
    }

    Ok(Command::SectionDef {
        name,
        duration_beats,
        entries,
        with_map: section_with,
    })
}

/// Items collected from a section body — either a single line or a nested repeat block.
#[derive(Debug)]
enum SectionBodyItem {
    Line(String),
    RepeatBlock { header: String, body: Vec<String> },
}

/// Parse a `pattern_ref` pair into a `PatternRef`.
fn parse_pattern_ref(pair: Pair<Rule>) -> Result<PatternRef> {
    match pair.as_rule() {
        Rule::pattern_ref => {
            let inner = pair.into_inner().next().unwrap();
            parse_pattern_ref(inner)
        }
        Rule::sample_call => {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let start: f64 = inner.next().unwrap().as_str().parse()?;
            let end: Option<f64> = inner.next().map(|p| p.as_str().parse()).transpose()?;
            Ok(PatternRef::Sample { name, start, end })
        }
        Rule::ident => {
            Ok(PatternRef::Name(pair.as_str().to_string()))
        }
        _ => Err(anyhow!("Unexpected rule in pattern_ref: {:?}", pair.as_rule())),
    }
}

/// Parse a single section entry line.
fn parse_section_entry(line: &str) -> Result<SectionEntry> {
    if line.starts_with("at ") {
        // at N play X ... or at N repeat X ...
        if line.contains(" play ") {
            // Check if this is a full inline expression (has "for N beats")
            // or a pattern reference (just a name)
            if line.contains(" for ") && line.contains(" beats") {
                // Full inline expression: at 0 play sine(440) >> lowpass(800) for 2 beats
                let at_pairs = ScoreParser::parse(Rule::at_stmt, line)
                    .map_err(|e| anyhow!("Section inline event parse error:\n{e}"))?;
                let at_pair = at_pairs.into_iter().next().unwrap();
                let parsed = parse_at(at_pair)?;
                if let Command::PlayAt { beat, expr, duration_beats, source, voice_label } = parsed {
                    return Ok(SectionEntry::InlineEvent { beat, expr, duration_beats, voice_label });
                }
                unreachable!()
            } else {
                // Pattern reference: at 8 play my_pattern
                let entry_pairs = ScoreParser::parse(Rule::section_entry_at_play, line)
                    .map_err(|e| anyhow!("Section entry parse error:\n{e}\n  Hint: 'at' inside sections positions a pattern by name: at 8 play my_pattern\n  For inline expressions, include 'for N beats': at 0 play sine(440) for 2 beats"))?;
                let entry_pair = entry_pairs.into_iter().next().unwrap();
                let mut ei = entry_pair.into_inner();
                let beat: f64 = ei.next().unwrap().as_str().parse()?;
                let pref = parse_pattern_ref(ei.next().unwrap())?;
                let entry_with = parse_optional_with(&mut ei);
                Ok(SectionEntry::AtPlay { beat, pattern: pref, with_map: entry_with })
            }
        } else if line.contains(" repeat ") {
            let entry_pairs = ScoreParser::parse(Rule::section_entry_at_repeat, line)
                .map_err(|e| anyhow!("Section entry parse error:\n{e}"))?;
            let entry_pair = entry_pairs.into_iter().next().unwrap();
            let mut ei = entry_pair.into_inner();
            let beat: f64 = ei.next().unwrap().as_str().parse()?;
            let pref = parse_pattern_ref(ei.next().unwrap())?;
            let (every, from, to) = parse_repeat_modifiers(&mut ei);
            let entry_with = parse_optional_with(&mut ei);
            Ok(SectionEntry::AtRepeat {
                beat,
                pattern: pref,
                every_beats: every,
                from_beat: from,
                to_beat: to,
                with_map: entry_with,
            })
        } else {
            Err(anyhow!(
                "Unrecognized section entry: {line}\n  Hint: Inside sections, use 'at N play <pattern_name>' or 'at N repeat <pattern_name> every N beats'.\n  If you want to play a raw expression, wrap it in a pattern first."
            ))
        }
    } else if line.starts_with("repeat ") {
        let entry_pairs = ScoreParser::parse(Rule::section_entry_repeat, line)
            .map_err(|e| anyhow!("Section repeat entry parse error:\n{e}"))?;
        let entry_pair = entry_pairs.into_iter().next().unwrap();
        let mut ei = entry_pair.into_inner();
        let pref = parse_pattern_ref(ei.next().unwrap())?;
        let (every, from, to) = parse_repeat_modifiers(&mut ei);
        let entry_with = parse_optional_with(&mut ei);
        Ok(SectionEntry::RepeatEvery {
            pattern: pref,
            every_beats: every,
            from_beat: from,
            to_beat: to,
            with_map: entry_with,
        })
    } else if line.starts_with("play ") {
        let entry_pairs = ScoreParser::parse(Rule::section_entry_play, line)
            .map_err(|e| anyhow!("Section play entry parse error:\n{e}"))?;
        let entry_pair = entry_pairs.into_iter().next().unwrap();
        let mut ei = entry_pair.into_inner();
        let pref = parse_pattern_ref(ei.next().unwrap())?;
        // Check for play_from modifier: "play melody from 8" => AtPlay
        let mut play_from_beat: Option<f64> = None;
        let mut remaining_pairs: Vec<Pair<Rule>> = Vec::new();
        for p in ei {
            if p.as_rule() == Rule::play_from {
                let num = p.into_inner().next().unwrap();
                play_from_beat = Some(num.as_str().parse()?);
            } else {
                remaining_pairs.push(p);
            }
        }
        let entry_with = {
            let mut iter = remaining_pairs.into_iter().peekable();
            let mut wm = None;
            for p in iter {
                if p.as_rule() == Rule::with_clause {
                    wm = parse_with_clause(p);
                }
            }
            wm
        };
        if let Some(beat) = play_from_beat {
            Ok(SectionEntry::AtPlay { beat, pattern: pref, with_map: entry_with })
        } else {
            Ok(SectionEntry::Play { pattern: pref, with_map: entry_with })
        }
    } else if line.starts_with("sequence ") {
        let entry_pairs = ScoreParser::parse(Rule::section_entry_sequence, line)
            .map_err(|e| anyhow!("Section sequence entry parse error:\n{e}"))?;
        let entry_pair = entry_pairs.into_iter().next().unwrap();
        let mut patterns = Vec::new();
        let mut entry_with = None;
        for token in entry_pair.into_inner() {
            match token.as_rule() {
                Rule::pattern_ref => {
                    patterns.push(parse_pattern_ref(token)?);
                }
                Rule::with_clause => entry_with = parse_with_clause(token),
                _ => {}
            }
        }
        Ok(SectionEntry::Sequence { patterns, with_map: entry_with })
    } else if line.starts_with("pattern ") {
        Err(anyhow!(
            "Can't nest a pattern inside a section.\n  Hint: Define the pattern separately (outside the section), then reference it by name:\n    pattern my_pat = 4 beats\n      at 0 play ... for ...\n    section my_section = 16 beats\n      repeat my_pat every 4 beats"
        ))
    } else {
        Err(anyhow!(
            "Unrecognized section entry: {line}\n  Hint: Section entries must be one of:\n    play <pattern_name>\n    repeat <pattern_name> [every N beats] [from N] [to N]\n    at N play <pattern_name>\n    at N repeat <pattern_name> [every N beats] [from N] [to N]\n    sequence <name1>, <name2>, ..."
        ))
    }
}

/// Parse repeat modifiers (every, from, to/until) from remaining pairs.
fn parse_repeat_modifiers(pairs: &mut pest::iterators::Pairs<Rule>) -> (Option<f64>, Option<f64>, Option<f64>) {
    let mut every = None;
    let mut from = None;
    let mut to = None;

    for pair in pairs {
        match pair.as_rule() {
            Rule::repeat_modifier => {
                let inner = pair.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::repeat_every => {
                        let num = inner.into_inner().next().unwrap();
                        every = Some(num.as_str().parse().unwrap_or(4.0));
                    }
                    Rule::repeat_from => {
                        let num = inner.into_inner().next().unwrap();
                        from = Some(num.as_str().parse().unwrap_or(0.0));
                    }
                    Rule::repeat_to => {
                        let num = inner.into_inner().next().unwrap();
                        to = Some(num.as_str().parse().unwrap_or(0.0));
                    }
                    _ => {}
                }
            }
            _ => break, // stop at with_clause or anything else
        }
    }

    (every, from, to)
}

/// Parse a with_clause pair into a WithMap.
fn parse_with_clause(pair: pest::iterators::Pair<Rule>) -> Option<WithMap> {
    let map = parse_mappings(pair.into_inner());
    if map.is_empty() { None } else { Some(map) }
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

fn parse_voice_def(pair: Pair<Rule>, kind: DefKind) -> Result<Command> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let expr = parse_expr(inner.next().unwrap())?;
    Ok(Command::VoiceDef { name, expr, kind })
}

fn parse_bpm(pair: Pair<Rule>) -> Result<Command> {
    let mut inner = pair.into_inner();
    let bpm: f64 = inner.next().unwrap().as_str().parse()?;
    Ok(Command::SetBpm { bpm, at_beat: None })
}

fn parse_at(pair: Pair<Rule>) -> Result<Command> {
    let mut inner = pair.into_inner();
    let beat: f64 = inner.next().unwrap().as_str().parse()?;
    let expr = parse_expr(inner.next().unwrap())?;
    let duration_beats: f64 = inner.next().unwrap().as_str().parse()?;
    let voice_label = extract_voice_label(&expr);
    Ok(Command::PlayAt {
        beat,
        expr,
        duration_beats,
        source: None,
        voice_label,
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
        Rule::chord_name => {
            // Chord names are kept as VoiceRef strings — resolve_chord handles them
            // in the graph builder when used in chord(), arp(), or instrument() calls.
            Ok(Expr::VoiceRef(pair.as_str().to_string()))
        }
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

/// Resolve a chord name to a vector of frequencies in Hz.
/// Returns None if the string isn't a valid chord name.
///
/// Format: Root[Accidental][Octave]:Quality
///   Examples: C:maj7, G3:7, Bb:m7, F#4:dim, A:m
///   Root: A-G
///   Accidental: # s b (optional)
///   Octave: 0-9 (optional, defaults to 4)
///   Quality: maj, m, min, dim, aug, 7, dom7, m7, min7, maj7, dim7, aug7,
///            9, dom9, m9, min9, maj9, sus2, sus4, m7b5, mmaj7, add9, 6, m6
pub fn resolve_chord(name: &str) -> Option<Vec<f64>> {
    // Split on colon — left is root+octave, right is quality
    let (root_part, quality) = name.split_once(':')?;

    if root_part.is_empty() || quality.is_empty() {
        return None;
    }

    let mut chars = root_part.chars();

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

    // Parse optional accidental and octave
    let rest: String = chars.collect();
    let (accidental, octave_str) = if rest.starts_with('#') || rest.starts_with('s') {
        (1i32, &rest[1..])
    } else if rest.starts_with('b') {
        (-1i32, &rest[1..])
    } else {
        (0i32, rest.as_str())
    };

    let octave: i32 = if octave_str.is_empty() {
        4 // default octave
    } else {
        octave_str.parse().ok()?
    };

    // Look up quality
    let qualities: &[(&str, &[i32])] = &[
        ("mmaj7", &[0, 3, 7, 11]),
        ("m7b5", &[0, 3, 6, 10]),
        ("maj9", &[0, 4, 7, 11, 14]),
        ("min9", &[0, 3, 7, 10, 14]),
        ("add9", &[0, 4, 7, 14]),
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
        ("m6", &[0, 3, 7, 9]),
        ("m", &[0, 3, 7]),
        ("9", &[0, 4, 7, 10, 14]),
        ("7", &[0, 4, 7, 10]),
        ("6", &[0, 4, 7, 9]),
    ];

    let intervals = qualities
        .iter()
        .find_map(|(suffix, ivs)| if *suffix == quality { Some(*ivs) } else { None })?;

    // Build the chord from root + octave + intervals
    let root_midi = (octave + 1) * 12 + semitone_base + accidental;
    let freqs: Vec<f64> = intervals
        .iter()
        .map(|iv| {
            let midi = root_midi + iv;
            440.0 * 2.0_f64.powf((midi as f64 - 69.0) / 12.0)
        })
        .collect();

    Some(freqs)
}

// Keep a legacy resolver for the generate module's Chord::parse which uses
// the old format internally (not user-facing).
pub fn resolve_chord_legacy(name: &str) -> Option<Vec<f64>> {
    // This handles the old format without colons, used by the generate module
    let mut chars = name.chars().peekable();
    let letter = chars.next()?;
    let semitone_base: i32 = match letter {
        'C' => 0, 'D' => 2, 'E' => 4, 'F' => 5,
        'G' => 7, 'A' => 9, 'B' => 11,
        _ => return None,
    };
    let rest: String = chars.collect();
    let (accidental, rest) = if rest.starts_with('#') || rest.starts_with('s') {
        (1i32, &rest[1..])
    } else if rest.starts_with('b') {
        (-1i32, &rest[1..])
    } else {
        (0i32, rest.as_str())
    };
    if rest.is_empty() { return None; }
    let qualities: &[(&str, &[i32])] = &[
        ("mmaj7", &[0, 3, 7, 11]), ("m7b5", &[0, 3, 6, 10]),
        ("maj9", &[0, 4, 7, 11, 14]), ("min9", &[0, 3, 7, 10, 14]),
        ("add9", &[0, 4, 7, 14]), ("maj7", &[0, 4, 7, 11]),
        ("min7", &[0, 3, 7, 10]), ("dim7", &[0, 3, 6, 9]),
        ("aug7", &[0, 4, 8, 10]), ("dom9", &[0, 4, 7, 10, 14]),
        ("dom7", &[0, 4, 7, 10]), ("sus2", &[0, 2, 7]),
        ("sus4", &[0, 5, 7]), ("min", &[0, 3, 7]),
        ("maj", &[0, 4, 7]), ("dim", &[0, 3, 6]),
        ("aug", &[0, 4, 8]), ("m9", &[0, 3, 7, 10, 14]),
        ("m7", &[0, 3, 7, 10]), ("m6", &[0, 3, 7, 9]),
        ("m", &[0, 3, 7]), ("9", &[0, 4, 7, 10, 14]),
        ("7", &[0, 4, 7, 10]), ("6", &[0, 4, 7, 9]),
    ];
    let (intervals, after_quality) = qualities
        .iter()
        .find_map(|(suffix, ivs)| rest.strip_prefix(suffix).map(|r| (*ivs, r)))?;
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
            Command::SetBpm { bpm, .. } => assert_eq!(*bpm, 120.0),
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
            Command::VoiceDef { name, expr, kind } => {
                assert_eq!(name, "x");
                assert_eq!(*kind, DefKind::Voice);
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
                swing,
                humanize,
            } => {
                assert_eq!(name, "drums");
                assert_eq!(*duration_beats, 4.0);
                assert_eq!(events.len(), 4);
                assert!(swing.is_none());
                assert!(humanize.is_none());
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
                ..
            } => {
                assert_eq!(name, "verse");
                assert_eq!(*duration_beats, Some(16.0));
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
            Command::PlaySequential { pattern } => assert_eq!(pattern.name(), "intro"),
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
        let notes = resolve_chord("C:m7").unwrap();
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
        let notes = resolve_chord("F:maj7").unwrap();
        assert_eq!(notes.len(), 4);
        let f4 = note_name_to_hz("F4").unwrap();
        let a4 = note_name_to_hz("A4").unwrap();
        assert!((notes[0] - f4).abs() < 0.01);
        assert!((notes[1] - a4).abs() < 0.01);
    }

    #[test]
    fn test_resolve_chord_am() {
        let notes = resolve_chord("A:m").unwrap();
        assert_eq!(notes.len(), 3); // minor triad
        let a4 = note_name_to_hz("A4").unwrap();
        assert!((notes[0] - a4).abs() < 0.01);
    }

    #[test]
    fn test_resolve_chord_g7() {
        // G:7 is G dominant 7th — no more ambiguity with G7 (note)
        let notes = resolve_chord("G:7").unwrap();
        assert_eq!(notes.len(), 4);
        let g4 = note_name_to_hz("G4").unwrap();
        let b4 = note_name_to_hz("B4").unwrap();
        assert!((notes[0] - g4).abs() < 0.01);
        assert!((notes[1] - b4).abs() < 0.01);
    }

    #[test]
    fn test_resolve_chord_bbm7() {
        let notes = resolve_chord("Bb:m7").unwrap();
        assert_eq!(notes.len(), 4);
        let bb4 = note_name_to_hz("Bb4").unwrap();
        assert!((notes[0] - bb4).abs() < 0.01);
    }

    #[test]
    fn test_resolve_chord_with_octave() {
        let notes3 = resolve_chord("C3:m7").unwrap();
        let notes4 = resolve_chord("C:m7").unwrap();
        // Octave 3 should be one octave below octave 4
        assert!((notes3[0] - notes4[0] / 2.0).abs() < 0.01);
    }

    #[test]
    fn test_resolve_chord_invalid() {
        assert!(resolve_chord("foo").is_none());
        assert!(resolve_chord("X:7").is_none());
        assert!(resolve_chord("C").is_none()); // bare letter, not a chord
        assert!(resolve_chord("Cm7").is_none()); // old format no longer works
    }
}
