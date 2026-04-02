use std::collections::HashMap;

use anyhow::{anyhow, Result};
use rand::Rng;

use super::ast::{Command, Expr, PatternEvent, PatternRef, RepeatBody, Script, SectionEntry, WithMap};
#[cfg(test)]
use super::ast::DefKind;

struct PatternInfo {
    duration_beats: f64,
    events: Vec<PatternEvent>,
    swing: Option<f64>,
    humanize: Option<f64>,
}

struct SectionInfo {
    duration_beats: Option<f64>,
    entries: Vec<SectionEntry>,
    with_map: Option<WithMap>,
}

/// Replace voice references in an expression according to a substitution map.
/// `with_map` maps pattern-local names to actual voice/instrument names.
fn apply_with_map(expr: &Expr, with_map: &WithMap) -> Expr {
    match expr {
        Expr::VoiceRef(name) => {
            if let Some(replacement) = with_map.get(name) {
                Expr::VoiceRef(replacement.clone())
            } else {
                expr.clone()
            }
        }
        Expr::FnCall { name, args } => {
            // Also substitute the function name if it's a voice/instrument invocation
            let resolved_name = with_map.get(name).cloned().unwrap_or_else(|| name.clone());
            Expr::FnCall {
                name: resolved_name,
                args: args.iter().map(|a| apply_with_map(a, with_map)).collect(),
            }
        }
        Expr::Pipe(a, b) => Expr::Pipe(
            Box::new(apply_with_map(a, with_map)),
            Box::new(apply_with_map(b, with_map)),
        ),
        Expr::Sum(a, b) => Expr::Sum(
            Box::new(apply_with_map(a, with_map)),
            Box::new(apply_with_map(b, with_map)),
        ),
        Expr::Sub(a, b) => Expr::Sub(
            Box::new(apply_with_map(a, with_map)),
            Box::new(apply_with_map(b, with_map)),
        ),
        Expr::Mul(a, b) => Expr::Mul(
            Box::new(apply_with_map(a, with_map)),
            Box::new(apply_with_map(b, with_map)),
        ),
        Expr::Div(a, b) => Expr::Div(
            Box::new(apply_with_map(a, with_map)),
            Box::new(apply_with_map(b, with_map)),
        ),
        Expr::Number(_) | Expr::Range(_, _) => expr.clone(),
    }
}

/// Merge with maps: inner values override outer values.
fn merge_with_maps(outer: &WithMap, inner: &WithMap) -> WithMap {
    let mut merged = outer.clone();
    merged.extend(inner.iter().map(|(k, v)| (k.clone(), v.clone())));
    merged
}

/// Extract the with_map from any section entry variant.
fn entry_with_map(entry: &SectionEntry) -> Option<&WithMap> {
    match entry {
        SectionEntry::RepeatEvery { with_map, .. }
        | SectionEntry::Play { with_map, .. }
        | SectionEntry::AtPlay { with_map, .. }
        | SectionEntry::AtRepeat { with_map, .. }
        | SectionEntry::Sequence { with_map, .. } => with_map.as_ref(),
        SectionEntry::RepeatBlock { .. } => None,
        SectionEntry::InlineEvent { .. } => None,
    }
}

/// Truncate a PlayAt event's duration to not exceed a boundary beat.
/// Non-PlayAt commands pass through unchanged.
fn truncate_to_boundary(cmd: Command, abs_end: f64) -> Command {
    if abs_end <= 0.0 {
        return cmd; // no boundary (implicit length sections)
    }
    if let Command::PlayAt { beat, expr, duration_beats, source, voice_label } = cmd {
        if beat >= abs_end {
            // Event starts past boundary — make it zero-length (will be inaudible)
            Command::PlayAt { beat, expr, duration_beats: 0.0, source, voice_label }
        } else {
            let clipped = duration_beats.min(abs_end - beat);
            Command::PlayAt { beat, expr, duration_beats: clipped, source, voice_label }
        }
    } else {
        cmd
    }
}

/// Resolve the effective with_map for a section entry.
fn resolve_entry_with(section_with: &WithMap, entry_with: Option<&WithMap>) -> WithMap {
    if let Some(ew) = entry_with {
        merge_with_maps(section_with, ew)
    } else {
        section_with.clone()
    }
}

struct ExpansionContext {
    patterns: HashMap<String, PatternInfo>,
    sections: HashMap<String, SectionInfo>,
}

impl ExpansionContext {
    fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            sections: HashMap::new(),
        }
    }

    fn duration_of(&self, name: &str) -> Result<f64> {
        if let Some(p) = self.patterns.get(name) {
            Ok(p.duration_beats)
        } else if let Some(s) = self.sections.get(name) {
            // If section has explicit duration, use it. Otherwise compute from entries.
            if let Some(d) = s.duration_beats {
                Ok(d)
            } else {
                self.compute_section_duration(&s.entries)
            }
        } else {
            Err(anyhow!("Unknown pattern or section: '{name}'"))
        }
    }

    /// Compute section duration from its entries (for implicit-length sections).
    fn compute_section_duration(&self, entries: &[SectionEntry]) -> Result<f64> {
        let mut max_end = 0.0_f64;
        let mut seq_cursor = 0.0_f64;

        for entry in entries {
            match entry {
                SectionEntry::RepeatEvery { pattern, to_beat, from_beat, every_beats, .. } => {
                    if let Some(to) = to_beat {
                        max_end = max_end.max(*to);
                    } else {
                        // Without a to_beat and no section duration, we need every_beats
                        let every = every_beats.unwrap_or_else(|| {
                            self.duration_of_ref(pattern).unwrap_or(4.0)
                        });
                        let from = from_beat.unwrap_or(0.0);
                        // Single tile if no upper bound
                        max_end = max_end.max(from + every);
                    }
                }
                SectionEntry::Play { pattern, .. } => {
                    let dur = self.duration_of_ref(pattern).unwrap_or(4.0);
                    seq_cursor += dur;
                    max_end = max_end.max(seq_cursor);
                }
                SectionEntry::AtPlay { beat, pattern, .. } => {
                    let dur = self.duration_of_ref(pattern).unwrap_or(4.0);
                    max_end = max_end.max(beat + dur);
                }
                SectionEntry::AtRepeat { beat, pattern, to_beat, every_beats, .. } => {
                    if let Some(to) = to_beat {
                        max_end = max_end.max(*to);
                    } else {
                        let every = every_beats.unwrap_or_else(|| {
                            self.duration_of_ref(pattern).unwrap_or(4.0)
                        });
                        max_end = max_end.max(beat + every);
                    }
                }
                SectionEntry::Sequence { patterns, .. } => {
                    let mut seq_total = 0.0;
                    for pref in patterns {
                        seq_total += self.duration_of_ref(pref).unwrap_or(4.0);
                    }
                    max_end = max_end.max(seq_total);
                }
                SectionEntry::InlineEvent { beat, duration_beats, .. } => {
                    max_end = max_end.max(beat + duration_beats);
                }
                SectionEntry::RepeatBlock { count, body } => {
                    // Estimate: count * average pattern duration
                    let mut body_dur = 0.0;
                    for item in body {
                        match item {
                            RepeatBody::Play(name) => body_dur += self.duration_of(name).unwrap_or(4.0),
                            RepeatBody::Pick(choices) => {
                                if let Some(c) = choices.first() {
                                    body_dur += self.duration_of(&c.name).unwrap_or(4.0);
                                }
                            }
                            RepeatBody::Shuffle(names) => {
                                for n in names {
                                    body_dur += self.duration_of(n).unwrap_or(4.0);
                                }
                            }
                        }
                    }
                    max_end = max_end.max(body_dur * *count as f64);
                }
            }
        }

        Ok(max_end)
    }

    fn expand_name(&self, name: &str, base_beat: f64, global_swing: f64, global_humanize: f64, bpm: f64, with_map: &WithMap, rng: &mut impl Rng) -> Result<Vec<Command>> {
        if let Some(p) = self.patterns.get(name) {
            let swing = p.swing.unwrap_or(global_swing);
            let humanize = p.humanize.unwrap_or(global_humanize);
            Ok(expand_pattern_events(&p.events, base_beat, swing, humanize, bpm, with_map, Some(name.to_string()), rng))
        } else if let Some(s) = self.sections.get(name) {
            self.expand_section(name, s, base_beat, global_swing, global_humanize, bpm, with_map, rng)
        } else {
            Err(anyhow!("Unknown pattern or section: '{name}'"))
        }
    }

    fn expand_pattern_ref(&self, pref: &PatternRef, base_beat: f64, swing: f64, humanize: f64, bpm: f64, with_map: &WithMap, rng: &mut impl Rng) -> Result<Vec<Command>> {
        match pref {
            PatternRef::Name(name) => self.expand_name(name, base_beat, swing, humanize, bpm, with_map, rng),
            PatternRef::Sample { name, start, end } => {
                let p = self.patterns.get(name)
                    .ok_or_else(|| anyhow!("Unknown pattern: '{name}' (referenced via sample())"))?;
                let pat_end = end.unwrap_or(p.duration_beats);
                let filtered: Vec<PatternEvent> = p.events.iter()
                    .filter(|e| e.beat_offset >= *start && e.beat_offset < pat_end)
                    .map(|e| PatternEvent {
                        beat_offset: e.beat_offset - start,
                        expr: e.expr.clone(),
                        duration_beats: e.duration_beats,
                    })
                    .collect();
                let pat_swing = p.swing.unwrap_or(swing);
                let pat_humanize = p.humanize.unwrap_or(humanize);
                Ok(expand_pattern_events(&filtered, base_beat, pat_swing, pat_humanize, bpm, with_map, Some(name.to_string()), rng))
            }
        }
    }

    fn duration_of_ref(&self, pref: &PatternRef) -> Result<f64> {
        match pref {
            PatternRef::Name(name) => self.duration_of(name),
            PatternRef::Sample { name, start, end } => {
                let p = self.patterns.get(name)
                    .ok_or_else(|| anyhow!("Unknown pattern: '{name}' (referenced via sample())"))?;
                Ok(end.unwrap_or(p.duration_beats) - start)
            }
        }
    }

    fn expand_section(&self, _section_name: &str, section: &SectionInfo, base_beat: f64, global_swing: f64, global_humanize: f64, bpm: f64, inherited_with: &WithMap, rng: &mut impl Rng) -> Result<Vec<Command>> {
        let mut output = Vec::new();

        // Merge inherited with_map with section-level with_map
        let section_with = if let Some(sw) = &section.with_map {
            merge_with_maps(inherited_with, sw)
        } else {
            inherited_with.clone()
        };

        // Resolve section duration (explicit or computed)
        let section_duration = section.duration_beats
            .unwrap_or_else(|| self.compute_section_duration(&section.entries).unwrap_or(0.0));

        for entry in &section.entries {
            let entry_with_map = entry_with_map(entry);
            let resolved_with = resolve_entry_with(&section_with, entry_with_map);

            match entry {
                SectionEntry::RepeatEvery { pattern, every_beats, from_beat, to_beat, .. } => {
                    let every = every_beats.unwrap_or_else(|| self.duration_of_ref(pattern).unwrap_or(4.0));
                    let start = from_beat.unwrap_or(0.0);
                    let end = to_beat.unwrap_or(section_duration);
                    let abs_end = base_beat + end;
                    let mut beat = start;
                    while beat < end {
                        let cmds = self.expand_pattern_ref(pattern, base_beat + beat, global_swing, global_humanize, bpm, &resolved_with, rng)?;
                        for cmd in cmds {
                            output.push(truncate_to_boundary(cmd, abs_end));
                        }
                        beat += every;
                    }
                }
                SectionEntry::Play { pattern, .. } => {
                    // Play starts at the section's base beat (simultaneous with other entries).
                    // Use `sequence` for back-to-back sequential playback.
                    let cmds = self.expand_pattern_ref(pattern, base_beat, global_swing, global_humanize, bpm, &resolved_with, rng)?;
                    // Truncate events to section boundary
                    let abs_end = base_beat + section_duration;
                    for cmd in cmds {
                        output.push(truncate_to_boundary(cmd, abs_end));
                    }
                }
                SectionEntry::AtPlay { beat, pattern, .. } => {
                    let cmds = self.expand_pattern_ref(pattern, base_beat + beat, global_swing, global_humanize, bpm, &resolved_with, rng)?;
                    let abs_end = base_beat + section_duration;
                    for cmd in cmds {
                        output.push(truncate_to_boundary(cmd, abs_end));
                    }
                }
                SectionEntry::AtRepeat { beat, pattern, every_beats, from_beat, to_beat, .. } => {
                    let every = every_beats.unwrap_or_else(|| self.duration_of_ref(pattern).unwrap_or(4.0));
                    let start = from_beat.unwrap_or(*beat);
                    let end = to_beat.unwrap_or(section_duration);
                    let abs_end = base_beat + end;
                    let mut b = start;
                    while b < end {
                        let cmds = self.expand_pattern_ref(pattern, base_beat + b, global_swing, global_humanize, bpm, &resolved_with, rng)?;
                        for cmd in cmds {
                            output.push(truncate_to_boundary(cmd, abs_end));
                        }
                        b += every;
                    }
                }
                SectionEntry::Sequence { patterns, .. } => {
                    let mut seq_cursor = 0.0;
                    for pref in patterns {
                        let name_with = resolve_entry_with(&section_with, entry_with_map);
                        let dur = self.duration_of_ref(pref).unwrap_or(4.0);
                        let cmds = self.expand_pattern_ref(pref, base_beat + seq_cursor, global_swing, global_humanize, bpm, &name_with, rng)?;
                        output.extend(cmds);
                        seq_cursor += dur;
                    }
                }
                SectionEntry::InlineEvent { beat, expr, duration_beats, voice_label } => {
                    // Apply with-map to the inline expression
                    let resolved_expr = if resolved_with.is_empty() {
                        expr.clone()
                    } else {
                        apply_with_map(expr, &resolved_with)
                    };
                    output.push(Command::PlayAt {
                        beat: base_beat + beat,
                        expr: resolved_expr,
                        duration_beats: *duration_beats,
                        source: None,
                        voice_label: voice_label.clone(),
                    });
                }
                SectionEntry::RepeatBlock { count, body } => {
                    let mut block_cursor = 0.0;
                    for _ in 0..*count {
                        for item in body {
                            match item {
                                RepeatBody::Play(name) => {
                                    let dur = self.duration_of(name)?;
                                    let cmds = self.expand_name(name, base_beat + block_cursor, global_swing, global_humanize, bpm, &resolved_with, rng)?;
                                    output.extend(cmds);
                                    block_cursor += dur;
                                }
                                RepeatBody::Pick(choices) => {
                                    let name = weighted_pick(choices, rng);
                                    let dur = self.duration_of(&name)?;
                                    let cmds = self.expand_name(&name, base_beat + block_cursor, global_swing, global_humanize, bpm, &resolved_with, rng)?;
                                    output.extend(cmds);
                                    block_cursor += dur;
                                }
                                RepeatBody::Shuffle(names) => {
                                    let mut shuffled = names.clone();
                                    shuffle_vec(&mut shuffled, rng);
                                    for name in &shuffled {
                                        let dur = self.duration_of(name)?;
                                        let cmds = self.expand_name(name, base_beat + block_cursor, global_swing, global_humanize, bpm, &resolved_with, rng)?;
                                        output.extend(cmds);
                                        block_cursor += dur;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(output)
    }
}

fn expand_pattern_events(events: &[PatternEvent], base_beat: f64, swing: f64, humanize_ms: f64, bpm: f64, with_map: &WithMap, source: Option<String>, rng: &mut impl Rng) -> Vec<Command> {
    events
        .iter()
        .map(|e| {
            let mut beat = base_beat + e.beat_offset;

            // Apply swing: shift events on offbeat positions
            if (swing - 0.5).abs() > 0.001 {
                let offset_frac = e.beat_offset.fract();
                // Swing any non-downbeat eighth-note position (0.5 within each beat)
                if (offset_frac - 0.5).abs() < 0.05 {
                    let offset_floor = e.beat_offset.floor();
                    beat = base_beat + offset_floor + swing;
                }
            }

            // Apply humanize: random timing jitter
            if humanize_ms > 0.0 {
                let jitter_beats = humanize_ms / 1000.0 * (bpm / 60.0);
                let r1: f64 = rng.gen_range(-1.0..1.0);
                let r2: f64 = rng.gen_range(-1.0..1.0);
                let offset = (r1 + r2) / 2.0 * jitter_beats;
                beat = (beat + offset).max(0.0);
            }

            // Capture original voice label BEFORE substitution
            let voice_label = crate::dsl::parser::extract_voice_label(&e.expr);

            // Apply voice substitution from with_map
            let expr = if with_map.is_empty() {
                e.expr.clone()
            } else {
                apply_with_map(&e.expr, with_map)
            };

            Command::PlayAt {
                beat,
                expr,
                duration_beats: e.duration_beats,
                source: source.clone(),
                voice_label,
            }
        })
        .collect()
}

/// Expand a script containing patterns, sections, repeat blocks, pick, and shuffle
/// into a flat script of VoiceDef, SetBpm, and PlayAt commands.
pub fn expand_script(script: Script, rng: &mut impl Rng) -> Result<Script> {
    let mut ctx = ExpansionContext::new();
    let mut output = Vec::new();
    let mut cursor: f64 = 0.0;
    let mut global_swing: f64 = 0.5; // 0.5 = straight
    let mut global_humanize: f64 = 0.0;
    let mut global_with: WithMap = HashMap::new();
    let mut bpm: f64 = 120.0;

    for cmd in script.commands {
        match cmd {
            Command::SetBpm { bpm: v, .. } => {
                bpm = v;
                // Annotate with current cursor position for tempo map
                output.push(Command::SetBpm { bpm: v, at_beat: Some(cursor) });
            }
            Command::SetSwing(v) => {
                global_swing = v;
                // consumed here — not passed to engine
            }
            Command::SetHumanize(v) => {
                global_humanize = v;
                // consumed here — not passed to engine
            }
            Command::SetWith(map) => {
                // Merge into global with_map (later statements override earlier ones)
                global_with.extend(map);
                // consumed here — not passed to engine
            }
            Command::VoiceDef { .. }
            | Command::WaveDef { .. }
            | Command::Import { .. }
            | Command::PedalDown { .. }
            | Command::PedalUp { .. }
            | Command::MasterCompress(_)
            | Command::MasterCeiling(_)
            | Command::MasterGain(_)
            | Command::MasterSaturate(_)
            | Command::MasterCurve { .. }
            | Command::MasterCurvePreset(_)
            | Command::MasterMultiband(_)
            | Command::Normalize { .. } => {
                output.push(cmd);
            }
            Command::PlayAt { .. } => {
                output.push(cmd);
            }
            Command::PatternDef {
                name,
                duration_beats,
                events,
                swing,
                humanize,
            } => {
                ctx.patterns
                    .insert(name, PatternInfo { duration_beats, events, swing, humanize });
            }
            Command::SectionDef {
                name,
                duration_beats,
                entries,
                with_map,
            } => {
                ctx.sections
                    .insert(name, SectionInfo { duration_beats, entries, with_map });
            }
            Command::PlaySequential { pattern } => {
                let duration = ctx.duration_of_ref(&pattern)?;
                let events = ctx.expand_pattern_ref(&pattern, cursor, global_swing, global_humanize, bpm, &global_with, rng)?;
                output.extend(events);
                cursor += duration;
            }
            Command::RepeatBlock { count, body } => {
                for _ in 0..count {
                    for item in &body {
                        match item {
                            RepeatBody::Play(name) => {
                                let duration = ctx.duration_of(name)?;
                                let events = ctx.expand_name(name, cursor, global_swing, global_humanize, bpm, &global_with, rng)?;
                                output.extend(events);
                                cursor += duration;
                            }
                            RepeatBody::Pick(choices) => {
                                let name = weighted_pick(choices, rng);
                                let duration = ctx.duration_of(&name)?;
                                let events = ctx.expand_name(&name, cursor, global_swing, global_humanize, bpm, &global_with, rng)?;
                                output.extend(events);
                                cursor += duration;
                            }
                            RepeatBody::Shuffle(names) => {
                                let mut shuffled = names.clone();
                                shuffle_vec(&mut shuffled, rng);
                                for name in &shuffled {
                                    let duration = ctx.duration_of(name)?;
                                    let events = ctx.expand_name(name, cursor, global_swing, global_humanize, bpm, &global_with, rng)?;
                                    output.extend(events);
                                    cursor += duration;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Script { commands: output })
}



fn weighted_pick(
    choices: &[super::ast::WeightedChoice],
    rng: &mut impl Rng,
) -> String {
    let total: f64 = choices.iter().map(|c| c.weight).sum();
    let mut r = rng.gen_range(0.0..total);
    for choice in choices {
        r -= choice.weight;
        if r <= 0.0 {
            return choice.name.clone();
        }
    }
    choices.last().unwrap().name.clone()
}

fn shuffle_vec(v: &mut [String], rng: &mut impl Rng) {
    let len = v.len();
    for i in (1..len).rev() {
        let j = rng.gen_range(0..=i);
        v.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::ast::*;
    use rand::SeedableRng;

    fn make_rng() -> rand::rngs::StdRng {
        rand::rngs::StdRng::seed_from_u64(42)
    }

    #[test]
    fn test_expand_pattern() {
        let script = Script {
            commands: vec![
                Command::VoiceDef {
                    name: "kick".into(),
                    expr: Expr::FnCall {
                        name: "sine".into(),
                        args: vec![Expr::Number(55.0)],
                    },
                    kind: DefKind::Voice,
                },
                Command::SetBpm { bpm: 120.0, at_beat: None },
                Command::PatternDef {
                    name: "drums".into(),
                    duration_beats: 4.0,
                    events: vec![
                        PatternEvent {
                            beat_offset: 0.0,
                            expr: Expr::VoiceRef("kick".into()),
                            duration_beats: 0.5,
                        },
                        PatternEvent {
                            beat_offset: 2.0,
                            expr: Expr::VoiceRef("kick".into()),
                            duration_beats: 0.5,
                        },
                    ],
                    swing: None,
                    humanize: None,
                },
                Command::PlaySequential {
                    pattern: PatternRef::Name("drums".into()),
                },
                Command::PlaySequential {
                    pattern: PatternRef::Name("drums".into()),
                },
            ],
        };

        let mut rng = make_rng();
        let expanded = expand_script(script, &mut rng).unwrap();

        // VoiceDef + SetBpm + 2 PlayAt (first drums) + 2 PlayAt (second drums) = 6
        assert_eq!(expanded.commands.len(), 6);

        // First pattern at cursor=0: beats 0.0 and 2.0
        match &expanded.commands[2] {
            Command::PlayAt { beat, .. } => assert_eq!(*beat, 0.0),
            _ => panic!("Expected PlayAt"),
        }
        match &expanded.commands[3] {
            Command::PlayAt { beat, .. } => assert_eq!(*beat, 2.0),
            _ => panic!("Expected PlayAt"),
        }

        // Second pattern at cursor=4: beats 4.0 and 6.0
        match &expanded.commands[4] {
            Command::PlayAt { beat, .. } => assert_eq!(*beat, 4.0),
            _ => panic!("Expected PlayAt"),
        }
        match &expanded.commands[5] {
            Command::PlayAt { beat, .. } => assert_eq!(*beat, 6.0),
            _ => panic!("Expected PlayAt"),
        }
    }

    #[test]
    fn test_expand_section() {
        let script = Script {
            commands: vec![
                Command::PatternDef {
                    name: "drums".into(),
                    duration_beats: 4.0,
                    events: vec![PatternEvent {
                        beat_offset: 0.0,
                        expr: Expr::VoiceRef("kick".into()),
                        duration_beats: 0.5,
                    }],
                    swing: None,
                    humanize: None,
                },
                Command::SectionDef {
                    name: "verse".into(),
                    duration_beats: Some(8.0),
                    entries: vec![SectionEntry::RepeatEvery {
                        pattern: PatternRef::Name("drums".into()),
                        every_beats: Some(4.0),
                        from_beat: None,
                        to_beat: None,
                        with_map: None,
                    }],
                    with_map: None,
                },
                Command::PlaySequential {
                    pattern: PatternRef::Name("verse".into()),
                },
            ],
        };

        let mut rng = make_rng();
        let expanded = expand_script(script, &mut rng).unwrap();

        // 2 PlayAt events (drums tiled at 0 and 4)
        assert_eq!(expanded.commands.len(), 2);
        match &expanded.commands[0] {
            Command::PlayAt { beat, .. } => assert_eq!(*beat, 0.0),
            _ => panic!("Expected PlayAt"),
        }
        match &expanded.commands[1] {
            Command::PlayAt { beat, .. } => assert_eq!(*beat, 4.0),
            _ => panic!("Expected PlayAt"),
        }
    }

    #[test]
    fn test_absolute_play_at_passes_through() {
        let script = Script {
            commands: vec![Command::PlayAt {
                beat: 10.0,
                expr: Expr::VoiceRef("x".into()),
                duration_beats: 2.0,
                source: None,
                voice_label: None,
            }],
        };

        let mut rng = make_rng();
        let expanded = expand_script(script, &mut rng).unwrap();
        assert_eq!(expanded.commands.len(), 1);
        match &expanded.commands[0] {
            Command::PlayAt { beat, .. } => assert_eq!(*beat, 10.0),
            _ => panic!("Expected PlayAt"),
        }
    }
}
