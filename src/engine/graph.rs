use std::collections::HashMap;

use anyhow::{anyhow, Result};
use fundsp::hacker::*;

use crate::dsl::ast::Expr;

/// Translate an AST Expr into a fundsp Net (boxed dynamic signal graph).
///
/// Voice definitions are looked up by name and recursively expanded,
/// so each call site gets its own independent DSP state.
///
/// `duration_secs` is the total play duration (from `PlayAt`), used by
/// time-aware envelopes like `swell(attack, release)`.
pub fn build_graph(
    expr: &Expr,
    voices: &HashMap<String, Expr>,
    sample_rate: f64,
    duration_secs: Option<f64>,
) -> Result<Net> {
    match expr {
        Expr::Number(v) => {
            let val = *v as f32;
            Ok(Net::wrap(Box::new(dc(val))))
        }

        Expr::FnCall { name, args } => {
            build_fn_call(name, args, voices, sample_rate, duration_secs)
        }

        Expr::VoiceRef(name) => {
            let voice_expr = voices
                .get(name)
                .ok_or_else(|| anyhow!("Unknown voice: {name}"))?;
            build_graph(voice_expr, voices, sample_rate, duration_secs)
        }

        Expr::Pipe(a, b) => {
            let net_a = build_graph(a, voices, sample_rate, duration_secs)?;
            let net_b = build_graph(b, voices, sample_rate, duration_secs)?;
            Ok(net_a >> net_b)
        }

        Expr::Sum(a, b) => {
            let net_a = build_graph(a, voices, sample_rate, duration_secs)?;
            let net_b = build_graph(b, voices, sample_rate, duration_secs)?;
            Ok(net_a + net_b)
        }

        Expr::Mul(a, b) => {
            let net_a = build_graph(a, voices, sample_rate, duration_secs)?;
            let net_b = build_graph(b, voices, sample_rate, duration_secs)?;
            Ok(net_a * net_b)
        }
    }
}

fn build_fn_call(
    name: &str,
    args: &[Expr],
    _voices: &HashMap<String, Expr>,
    sample_rate: f64,
    _duration_secs: Option<f64>,
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

        // Envelope: 1 input, 1 output — multiplies signal by exp(-rate * t)
        "decay" => {
            let rate = expect_number(&args, 0, name)?;
            let env = Net::wrap(Box::new(envelope(move |t: f64| {
                (-t * rate).exp()
            })));
            let pass_through = Net::wrap(Box::new(pass()));
            let mut net = pass_through * env;
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // Swell envelope: handled in the engine render loop for precise timing.
        // In the DSP graph, swell is just a passthrough.
        "swell" => {
            let mut net = Net::wrap(Box::new(pass()));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // LFO / tremolo: amplitude modulation at a given rate and depth.
        // depth is 0..1 — at depth=1 the signal swings from silence to full.
        "lfo" => {
            let rate = expect_number(args, 0, name)?;
            let depth = expect_number(args, 1, name)?;
            let env = Net::wrap(Box::new(envelope(move |t: f64| {
                1.0 - depth + depth * (t * rate * std::f64::consts::TAU).sin()
            })));
            let pass_through = Net::wrap(Box::new(pass()));
            let mut net = pass_through * env;
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // Distortion: tanh soft-clipping, normalized to preserve peak level.
        // amount ~1 = subtle warmth, ~5 = heavy saturation.
        "distort" => {
            let amount = expect_number(args, 0, name)? as f32;
            let norm = amount.tanh();
            let mut net = Net::wrap(Box::new(shape_fn(move |x: f32| {
                (x * amount).tanh() / norm
            })));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // Vibrato: pitch modulation via a modulated delay line.
        // rate = LFO frequency in Hz, depth_cents = pitch excursion in cents.
        "vibrato" => {
            let rate = expect_number(args, 0, name)? as f32;
            let depth_cents = expect_number(args, 1, name)? as f32;
            // Convert cents to delay modulation depth.
            // For small pitch deviations, Δt ≈ cents / (1200 * freq).
            // We use a fixed average delay with modulation around it.
            let max_delay: f32 = 0.03; // 30 ms buffer
            let avg_delay: f32 = max_delay / 2.0;
            // cents → fractional pitch ratio → delay excursion
            let depth: f32 = avg_delay * (2.0_f32.powf(depth_cents / 1200.0) - 1.0);
            // Build: input signal + LFO-modulated delay time → tap
            let lfo_signal = Net::wrap(Box::new(
                dc(avg_delay) + sine_hz(rate) * dc(depth),
            ));
            let input = Net::wrap(Box::new(pass()));
            let tap_node = Net::wrap(Box::new(tap(
                (avg_delay - depth).max(0.001),
                avg_delay + depth + 0.001,
            )));
            let mut net = (input | lfo_signal) >> tap_node;
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // Chorus: fundsp's built-in mono chorus effect.
        // separation = base delay, variation = modulation depth, mod_freq = LFO rate.
        "chorus" => {
            let separation = expect_number(args, 0, name)? as f32;
            let variation = expect_number(args, 1, name)? as f32;
            let mod_freq = expect_number(args, 2, name)? as f32;
            let mut net = Net::wrap(Box::new(chorus(0, separation, variation, mod_freq)));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        _ => Err(anyhow!("Unknown DSP function: {name}")),
    }
}

/// Walk an expression tree and extract swell(attack, release) parameters if present.
/// Swell is expected at the end of a pipe chain: `voice >> swell(a, r)`.
pub fn extract_swell(expr: &Expr) -> Option<(f64, f64)> {
    match expr {
        Expr::FnCall { name, args } if name == "swell" => {
            let attack = if let Some(Expr::Number(v)) = args.first() {
                *v
            } else {
                return None;
            };
            let release = if let Some(Expr::Number(v)) = args.get(1) {
                *v
            } else {
                return None;
            };
            Some((attack, release))
        }
        Expr::Pipe(_, b) => extract_swell(b),
        _ => None,
    }
}

/// Strip swell() from an expression tree, returning the expression without it.
/// This avoids adding an unnecessary pass() node to the DSP graph.
pub fn strip_swell(expr: &Expr) -> Expr {
    match expr {
        Expr::Pipe(a, b) => {
            if matches!(b.as_ref(), Expr::FnCall { name, .. } if name == "swell") {
                // Swell is at the end — return just the left side
                strip_swell(a)
            } else {
                Expr::Pipe(Box::new(strip_swell(a)), Box::new(strip_swell(b)))
            }
        }
        other => other.clone(),
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
