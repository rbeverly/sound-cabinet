use std::collections::HashMap;

use anyhow::{anyhow, Result};
use fundsp::hacker::*;

use crate::dsl::ast::Expr;

/// Translate an AST Expr into a fundsp Net (boxed dynamic signal graph).
///
/// Voice definitions are looked up by name and recursively expanded,
/// so each call site gets its own independent DSP state.
pub fn build_graph(expr: &Expr, voices: &HashMap<String, Expr>, sample_rate: f64) -> Result<Net> {
    match expr {
        Expr::Number(v) => {
            let val = *v as f32;
            Ok(Net::wrap(Box::new(dc(val))))
        }

        Expr::FnCall { name, args } => build_fn_call(name, args, voices, sample_rate),

        Expr::VoiceRef(name) => {
            let voice_expr = voices
                .get(name)
                .ok_or_else(|| anyhow!("Unknown voice: {name}"))?;
            build_graph(voice_expr, voices, sample_rate)
        }

        Expr::Pipe(a, b) => {
            let net_a = build_graph(a, voices, sample_rate)?;
            let net_b = build_graph(b, voices, sample_rate)?;
            Ok(net_a >> net_b)
        }

        Expr::Sum(a, b) => {
            let net_a = build_graph(a, voices, sample_rate)?;
            let net_b = build_graph(b, voices, sample_rate)?;
            Ok(net_a + net_b)
        }

        Expr::Mul(a, b) => {
            let net_a = build_graph(a, voices, sample_rate)?;
            let net_b = build_graph(b, voices, sample_rate)?;
            Ok(net_a * net_b)
        }
    }
}

fn build_fn_call(
    name: &str,
    args: &[Expr],
    _voices: &HashMap<String, Expr>,
    sample_rate: f64,
) -> Result<Net> {
    match name {
        // Oscillators: 0 inputs, 1 output
        "sine" => {
            let freq = expect_number(&args, 0, name)? as f32;
            let mut net = Net::wrap(Box::new(sine_hz(freq)));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }
        "saw" => {
            let freq = expect_number(&args, 0, name)? as f32;
            let mut net = Net::wrap(Box::new(saw_hz(freq)));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }
        "triangle" => {
            let freq = expect_number(&args, 0, name)? as f32;
            let mut net = Net::wrap(Box::new(triangle_hz(freq)));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }
        "square" => {
            let freq = expect_number(&args, 0, name)? as f32;
            let mut net = Net::wrap(Box::new(square_hz(freq)));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }
        "noise" => {
            let mut net = Net::wrap(Box::new(white()));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // Filters: 1 input, 1 output
        "lowpass" => {
            let freq = expect_number(&args, 0, name)? as f32;
            let q = expect_number(&args, 1, name)? as f32;
            let mut net = Net::wrap(Box::new(lowpass_hz(freq, q)));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }
        "highpass" => {
            let freq = expect_number(&args, 0, name)? as f32;
            let q = expect_number(&args, 1, name)? as f32;
            let mut net = Net::wrap(Box::new(highpass_hz(freq, q)));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        _ => Err(anyhow!("Unknown DSP function: {name}")),
    }
}

/// Extract a numeric literal from an argument list.
fn expect_number(args: &[Expr], index: usize, fn_name: &str) -> Result<f64> {
    match args.get(index) {
        Some(Expr::Number(v)) => Ok(*v),
        Some(_) => Err(anyhow!(
            "{fn_name}: argument {index} must be a number literal"
        )),
        None => Err(anyhow!(
            "{fn_name}: expected at least {} arguments",
            index + 1
        )),
    }
}
