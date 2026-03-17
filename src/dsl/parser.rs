use anyhow::{anyhow, Result};
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use super::ast::{Command, Expr, Script};

#[derive(Parser)]
#[grammar = "dsl/grammar.pest"]
pub struct ScoreParser;

/// Parse an entire score file into a Script.
pub fn parse_script(input: &str) -> Result<Script> {
    let pairs = ScoreParser::parse(Rule::script, input)
        .map_err(|e| anyhow!("Parse error:\n{e}"))?;

    let mut commands = Vec::new();

    for pair in pairs {
        if pair.as_rule() == Rule::script {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::line {
                    if let Some(cmd) = parse_line_pair(inner)? {
                        commands.push(cmd);
                    }
                }
            }
        }
    }

    Ok(Script { commands })
}

/// Parse a single line of input into a Command.
pub fn parse_line(input: &str) -> Result<Command> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return Err(anyhow!("Empty or comment line"));
    }

    // Try each statement type
    if let Ok(pairs) = ScoreParser::parse(Rule::voice_def, trimmed) {
        for pair in pairs {
            return parse_voice_def(pair);
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

fn parse_line_pair(pair: Pair<Rule>) -> Result<Option<Command>> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::voice_def => return Ok(Some(parse_voice_def(inner)?)),
            Rule::bpm_stmt => return Ok(Some(parse_bpm(inner)?)),
            Rule::at_stmt => return Ok(Some(parse_at(inner)?)),
            _ => {}
        }
    }
    Ok(None)
}

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
    // beat_unit is consumed but we don't need its value
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
        Rule::number => Ok(Expr::Number(pair.as_str().parse()?)),
        Rule::ident => Ok(Expr::VoiceRef(pair.as_str().to_string())),
        _ => Err(anyhow!("Unexpected rule: {:?}", pair.as_rule())),
    }
}

fn parse_binary_expr(pair: Pair<Rule>) -> Result<Expr> {
    let rule = pair.as_rule();
    let mut inner = pair.into_inner();

    let first = inner.next().unwrap();
    let mut left = parse_expr(first)?;

    while let Some(next) = inner.next() {
        let right = parse_expr(next)?;
        left = match rule {
            Rule::pipe_expr | Rule::expr => Expr::Pipe(Box::new(left), Box::new(right)),
            Rule::sum_expr => Expr::Sum(Box::new(left), Box::new(right)),
            Rule::mul_expr => Expr::Mul(Box::new(left), Box::new(right)),
            _ => unreachable!(),
        };
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
        let cmd = parse_line("bpm 120").unwrap();
        match cmd {
            Command::SetBpm(bpm) => assert_eq!(bpm, 120.0),
            _ => panic!("Expected SetBpm"),
        }
    }

    #[test]
    fn test_parse_voice_def() {
        let cmd = parse_line("voice pad = sine(440)").unwrap();
        match cmd {
            Command::VoiceDef { name, .. } => assert_eq!(name, "pad"),
            _ => panic!("Expected VoiceDef"),
        }
    }

    #[test]
    fn test_parse_at() {
        let cmd = parse_line("at 0 play sine(440) for 2 beats").unwrap();
        match cmd {
            Command::PlayAt {
                beat,
                duration_beats,
                ..
            } => {
                assert_eq!(beat, 0.0);
                assert_eq!(duration_beats, 2.0);
            }
            _ => panic!("Expected PlayAt"),
        }
    }

    #[test]
    fn test_parse_complex_expr() {
        let cmd = parse_line("voice x = (saw(40) + 0.5 * sine(80)) >> lowpass(2000, 0.7)").unwrap();
        match cmd {
            Command::VoiceDef { name, expr } => {
                assert_eq!(name, "x");
                match expr {
                    Expr::Pipe(_, _) => {} // good — top level is pipe
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
}
