use std::collections::HashMap;

use anyhow::{anyhow, Result};
use rand::Rng;

use super::ast::{Command, Expr, PatternEvent, RepeatBody, Script, SectionEntry, WithMap};
#[cfg(test)]
use super::ast::DefKind;

struct PatternInfo {
    duration_beats: f64,
    events: Vec<PatternEvent>,
    swing: Option<f64>,
    humanize: Option<f64>,
}

struct SectionInfo {
    duration_beats: f64,
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
            Ok(s.duration_beats)
        } else {
            Err(anyhow!("Unknown pattern or section: '{name}'"))
        }
    }

    fn expand_name(&self, name: &str, base_beat: f64, global_swing: f64, global_humanize: f64, bpm: f64, with_map: &WithMap, rng: &mut impl Rng) -> Result<Vec<Command>> {
        if let Some(p) = self.patterns.get(name) {
            let swing = p.swing.unwrap_or(global_swing);
            let humanize = p.humanize.unwrap_or(global_humanize);
            Ok(expand_pattern_events(&p.events, base_beat, swing, humanize, bpm, with_map, rng))
        } else if let Some(s) = self.sections.get(name) {
            self.expand_section(s, base_beat, global_swing, global_humanize, bpm, with_map, rng)
        } else {
            Err(anyhow!("Unknown pattern or section: '{name}'"))
        }
    }

    fn expand_section(&self, section: &SectionInfo, base_beat: f64, global_swing: f64, global_humanize: f64, bpm: f64, inherited_with: &WithMap, rng: &mut impl Rng) -> Result<Vec<Command>> {
        let mut output = Vec::new();

        // Merge inherited with_map with section-level with_map
        let section_with = if let Some(sw) = &section.with_map {
            merge_with_maps(inherited_with, sw)
        } else {
            inherited_with.clone()
        };

        for entry in &section.entries {
            match entry {
                SectionEntry::RepeatEvery { name, every_beats, with_map } => {
                    // Merge section-level with entry-level (entry overrides section)
                    let entry_with = if let Some(ew) = with_map {
                        merge_with_maps(&section_with, ew)
                    } else {
                        section_with.clone()
                    };
                    let mut beat = 0.0;
                    while beat < section.duration_beats {
                        let cmds = self.expand_name(name, base_beat + beat, global_swing, global_humanize, bpm, &entry_with, rng)?;
                        output.extend(cmds);
                        beat += every_beats;
                    }
                }
                SectionEntry::Play { name, with_map } => {
                    let entry_with = if let Some(ew) = with_map {
                        merge_with_maps(&section_with, ew)
                    } else {
                        section_with.clone()
                    };
                    let cmds = self.expand_name(name, base_beat, global_swing, global_humanize, bpm, &entry_with, rng)?;
                    output.extend(cmds);
                }
            }
        }

        Ok(output)
    }
}

fn expand_pattern_events(events: &[PatternEvent], base_beat: f64, swing: f64, humanize_ms: f64, bpm: f64, with_map: &WithMap, rng: &mut impl Rng) -> Vec<Command> {
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
            | Command::MasterCeiling(_) => {
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
            Command::PlaySequential { name } => {
                let duration = ctx.duration_of(&name)?;
                let events = ctx.expand_name(&name, cursor, global_swing, global_humanize, bpm, &global_with, rng)?;
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
                    name: "drums".into(),
                },
                Command::PlaySequential {
                    name: "drums".into(),
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
                    duration_beats: 8.0,
                    entries: vec![SectionEntry::RepeatEvery {
                        name: "drums".into(),
                        every_beats: 4.0,
                        with_map: None,
                    }],
                    with_map: None,
                },
                Command::PlaySequential {
                    name: "verse".into(),
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
