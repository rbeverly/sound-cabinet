use std::collections::HashMap;

use anyhow::{anyhow, Result};
use fundsp::hacker::*;

use crate::dsl::ast::Expr;
use crate::dsl::parser::resolve_chord;
use crate::engine::effects::{BitCrush, Compressor, Decimate, Degrade, FeedbackDelay, Freeverb, LeakyFilter, WavetableOsc};

const MAX_RECURSION_DEPTH: usize = 64;

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
    wavetables: &HashMap<String, Vec<f64>>,
    sample_rate: f64,
    duration_secs: Option<f64>,
) -> Result<Net> {
    build_graph_inner(expr, voices, wavetables, sample_rate, duration_secs, 0)
}

fn build_graph_inner(
    expr: &Expr,
    voices: &HashMap<String, Expr>,
    wavetables: &HashMap<String, Vec<f64>>,
    sample_rate: f64,
    duration_secs: Option<f64>,
    depth: usize,
) -> Result<Net> {
    if depth > MAX_RECURSION_DEPTH {
        return Err(anyhow!("Maximum recursion depth exceeded — possible circular voice reference"));
    }
    match expr {
        Expr::Number(v) => {
            let val = *v as f32;
            Ok(Net::wrap(Box::new(dc(val))))
        }

        Expr::Range(start, end) => {
            // Range as a standalone expression: produces a time-varying dc signal
            let start = *start;
            let end = *end;
            let dur = duration_secs.unwrap_or(4.0);
            let mut net = Net::wrap(Box::new(envelope(move |t: f64| {
                let frac = (t / dur).min(1.0);
                start + (end - start) * frac
            })));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        Expr::FnCall { name, args } => {
            build_fn_call(name, args, voices, wavetables, sample_rate, duration_secs, depth)
        }

        Expr::VoiceRef(name) => {
            let voice_expr = voices
                .get(name)
                .ok_or_else(|| anyhow!("Unknown voice: {name}"))?;
            build_graph_inner(voice_expr, voices, wavetables, sample_rate, duration_secs, depth + 1)
        }

        Expr::Pipe(a, b) => {
            let net_a = build_graph_inner(a, voices, wavetables, sample_rate, duration_secs, depth + 1)?;
            let net_b = build_graph_inner(b, voices, wavetables, sample_rate, duration_secs, depth + 1)?;
            Ok(net_a >> net_b)
        }

        Expr::Sum(a, b) => {
            let net_a = build_graph_inner(a, voices, wavetables, sample_rate, duration_secs, depth + 1)?;
            let net_b = build_graph_inner(b, voices, wavetables, sample_rate, duration_secs, depth + 1)?;
            Ok(net_a + net_b)
        }

        Expr::Mul(a, b) => {
            let net_a = build_graph_inner(a, voices, wavetables, sample_rate, duration_secs, depth + 1)?;
            let net_b = build_graph_inner(b, voices, wavetables, sample_rate, duration_secs, depth + 1)?;
            Ok(net_a * net_b)
        }

        Expr::Sub(a, b) => {
            let net_a = build_graph_inner(a, voices, wavetables, sample_rate, duration_secs, depth + 1)?;
            let net_b = build_graph_inner(b, voices, wavetables, sample_rate, duration_secs, depth + 1)?;
            Ok(net_a - net_b)
        }

        Expr::Div(_, _) => {
            Err(anyhow!("Division of signal graphs is not supported — use in instrument definitions where it constant-folds (e.g., 1000 / freq)"))
        }
    }
}

fn build_fn_call(
    name: &str,
    args: &[Expr],
    voices: &HashMap<String, Expr>,
    wavetables: &HashMap<String, Vec<f64>>,
    sample_rate: f64,
    duration_secs: Option<f64>,
    depth: usize,
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
        "pulse" => {
            let freq = expect_number(&args, 0, name)? as f32;
            let freq_dc = Net::wrap(Box::new(dc(freq)));

            // Width can be static or a sweep (pulse width modulation)
            let width_signal = if let Some(Expr::Range(start, end)) = args.get(1) {
                let start = *start;
                let end = *end;
                let dur = duration_secs.unwrap_or(4.0);
                Net::wrap(Box::new(envelope(move |t: f64| {
                    let frac = (t / dur).min(1.0);
                    start + (end - start) * frac
                })))
            } else {
                let width = if args.len() > 1 {
                    expect_number(&args, 1, name)? as f32
                } else {
                    0.5 // default 50% = square wave
                };
                Net::wrap(Box::new(dc(width)))
            };

            // pulse() takes 2 inputs: frequency, duty cycle
            let osc = Net::wrap(Box::new(pulse()));
            let mut net = (freq_dc | width_signal) >> osc;
            net.set_sample_rate(sample_rate);
            Ok(net)
        }
        "noise" | "white" => {
            let mut net = Net::wrap(Box::new(white()));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }
        "pink" => {
            let mut net = Net::wrap(Box::new(pink()));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }
        "brown" => {
            let mut net = Net::wrap(Box::new(brown()));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // Filters: 1 input, 1 output (with optional parameter automation and mix)
        // lowpass(freq, q) or lowpass(freq, q, mix) where mix 0.0-1.0 blends dry/wet
        "lowpass" => {
            let q = expect_number(&args, 1, name)? as f32;
            let mix: Option<f32> = if args.len() > 2 {
                Some(expect_number(&args, 2, name)? as f32)
            } else {
                None
            };

            if let Some(mix_val) = mix {
                // Use LeakyFilter: custom one-pole lowpass with internal dry/wet blend
                let freq = expect_number(&args, 0, name)? as f32;
                let mut net = Net::wrap(Box::new(An(LeakyFilter::new(freq, mix_val))));
                net.set_sample_rate(sample_rate);
                Ok(net)
            } else if let Some(Expr::Range(start, end)) = args.first() {
                // Dynamic lowpass: (signal | cutoff_sweep) >> lowpass_q(q)
                let start = *start;
                let end = *end;
                let dur = duration_secs.unwrap_or(4.0);
                let sweep = Net::wrap(Box::new(envelope(move |t: f64| {
                    let frac = (t / dur).min(1.0);
                    start + (end - start) * frac
                })));
                let input = Net::wrap(Box::new(pass()));
                let filter = Net::wrap(Box::new(lowpass_q(q)));
                let mut net = (input | sweep) >> filter;
                net.set_sample_rate(sample_rate);
                Ok(net)
            } else {
                // Static lowpass (original behavior)
                let freq = expect_number(&args, 0, name)? as f32;
                let mut net = Net::wrap(Box::new(lowpass_hz(freq, q)));
                net.set_sample_rate(sample_rate);
                Ok(net)
            }
        }
        "highpass" => {
            let q = expect_number(&args, 1, name)? as f32;

            if let Some(Expr::Range(start, end)) = args.first() {
                let start = *start;
                let end = *end;
                let dur = duration_secs.unwrap_or(4.0);
                let sweep = Net::wrap(Box::new(envelope(move |t: f64| {
                    let frac = (t / dur).min(1.0);
                    start + (end - start) * frac
                })));
                let input = Net::wrap(Box::new(pass()));
                let lp_filter = Net::wrap(Box::new(lowpass_q(q)));
                let lp_path = (input | sweep) >> lp_filter;
                let original = Net::wrap(Box::new(pass()));
                let mut net = original - lp_path;
                net.set_sample_rate(sample_rate);
                Ok(net)
            } else {
                let freq = expect_number(&args, 0, name)? as f32;
                let mut net = Net::wrap(Box::new(highpass_hz(freq, q)));
                net.set_sample_rate(sample_rate);
                Ok(net)
            }
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

        // Delay: feedback delay line with automatic damping.
        // delay(time_seconds, feedback_0_to_1, mix_0_to_1)
        "delay" => {
            let time = expect_number(args, 0, name)?;
            let feedback = expect_number(args, 1, name)? as f32;
            let mix = expect_number(args, 2, name)? as f32;
            let mut net = Net::wrap(Box::new(An(FeedbackDelay::new(time, feedback, mix))));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // Reverb: Freeverb algorithm — 8 comb filters + 4 allpass diffusers.
        // reverb(room_size_0_to_1, damping_0_to_1, mix_0_to_1)
        "reverb" => {
            let room_size = expect_number(args, 0, name)? as f32;
            let damping = expect_number(args, 1, name)? as f32;
            let mix = expect_number(args, 2, name)? as f32;
            let mut net = Net::wrap(Box::new(An(Freeverb::new(room_size, damping, mix))));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // Compressor: dynamic range compression
        // compress(threshold_db, ratio, attack_secs, release_secs)
        // e.g., compress(-20, 4, 0.01, 0.1)
        "compress" => {
            let threshold = expect_number(args, 0, name)? as f32;
            let ratio = expect_number(args, 1, name)? as f32;
            let attack = if args.len() > 2 {
                expect_number(args, 2, name)?
            } else {
                0.01 // 10ms default attack
            };
            let release = if args.len() > 3 {
                expect_number(args, 3, name)?
            } else {
                0.1 // 100ms default release
            };
            let mut net = Net::wrap(Box::new(An(Compressor::new(threshold, ratio, attack, release))));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // Bit crusher: reduce bit depth for quantization noise
        // crush(bits) — 8.0 = retro, 4.0 = heavy, 12.0 = subtle
        "crush" => {
            let bits = expect_number(args, 0, name)? as f32;
            let mut net = Net::wrap(Box::new(An(BitCrush::new(bits))));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // Decimator: reduce effective sample rate for aliasing artifacts
        // decimate(factor) — 2.0 = half rate, 8.0 = heavy, 1.0 = no effect
        "decimate" => {
            let factor = expect_number(args, 0, name)? as f32;
            let mut net = Net::wrap(Box::new(An(Decimate::new(factor))));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // Degrade: combined tape/medium degradation
        // degrade(amount) — 0.0 = clean, 0.3 = warm, 0.6 = worn tape, 1.0 = destroyed
        "degrade" => {
            let amount = expect_number(args, 0, name)? as f32;
            let mut net = Net::wrap(Box::new(An(Degrade::new(amount))));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // Chord: play all notes of a named chord simultaneously.
        // chord(Cm7) — generates summed saw oscillators scaled by 1/N.
        "chord" => {
            let chord_name = match args.first() {
                Some(Expr::VoiceRef(name)) => name.clone(),
                _ => {
                    return Err(anyhow!(
                        "chord: argument must be a chord name like Cm7, Am, Fmaj7"
                    ))
                }
            };
            let notes = resolve_chord(&chord_name)
                .ok_or_else(|| anyhow!("chord: unknown chord '{chord_name}'"))?;
            let scale = 1.0 / notes.len() as f32;
            let mut net = Net::wrap(Box::new(dc(0.0)));
            net.set_sample_rate(sample_rate);
            for freq in &notes {
                let mut osc = Net::wrap(Box::new(saw_hz(*freq as f32)));
                osc.set_sample_rate(sample_rate);
                let mut gain = Net::wrap(Box::new(dc(scale)));
                gain.set_sample_rate(sample_rate);
                net = net + (osc * gain);
            }
            Ok(net)
        }

        // Wavetable oscillator: name(freq) where name is a defined wave
        _ if wavetables.contains_key(name) => {
            let samples = &wavetables[name];
            let freq = expect_number(args, 0, name)? as f32;
            let mut net = Net::wrap(Box::new(An(WavetableOsc::new(samples, freq))));
            net.set_sample_rate(sample_rate);
            Ok(net)
        }

        // Instrument invocation: name(freq) or name(ChordName)
        _ => {
            if let Some(template) = voices.get(name) {
                // Check if the argument is a chord name → sum instrument for each note
                if let Some(Expr::VoiceRef(chord_name)) = args.first() {
                    if let Some(chord_notes) = resolve_chord(chord_name) {
                        let scale = 1.0 / chord_notes.len() as f32;
                        let mut net = Net::wrap(Box::new(dc(0.0)));
                        net.set_sample_rate(sample_rate);
                        for freq in &chord_notes {
                            let substituted = substitute_var(template, "freq", *freq);
                            let note_net = build_graph_inner(&substituted, voices, wavetables, sample_rate, duration_secs, depth + 1)?;
                            let mut gain = Net::wrap(Box::new(dc(scale)));
                            gain.set_sample_rate(sample_rate);
                            net = net + (note_net * gain);
                        }
                        return Ok(net);
                    }
                }
                // Single frequency
                let freq = expect_number(args, 0, name)?;
                let substituted = substitute_var(template, "freq", freq);
                build_graph_inner(&substituted, voices, wavetables, sample_rate, duration_secs, depth + 1)
            } else {
                Err(anyhow!("Unknown DSP function: {name}"))
            }
        }
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

/// Flatten a left-associative pipe chain into a vec of segments.
/// `a >> b >> c` (parsed as `Pipe(Pipe(a, b), c)`) becomes `[a, b, c]`.
fn flatten_pipe_chain(expr: &Expr) -> Vec<Expr> {
    match expr {
        Expr::Pipe(a, b) => {
            let mut v = flatten_pipe_chain(a);
            v.extend(flatten_pipe_chain(b));
            v
        }
        other => vec![other.clone()],
    }
}

/// Rebuild a pipe chain from segments: `[a, b, c]` becomes `a >> b >> c`.
fn build_pipe_chain(segments: &[Expr]) -> Option<Expr> {
    if segments.is_empty() {
        None
    } else {
        let mut result = segments[0].clone();
        for seg in &segments[1..] {
            result = Expr::Pipe(Box::new(result), Box::new(seg.clone()));
        }
        Some(result)
    }
}

/// Find an `arp(...)` call anywhere in a pipe chain and split the chain into
/// (pre_chain, arp_args, post_chain). Returns None if no arp is present.
///
/// For `pluck >> arp(C4, E4, G4, 4) >> lowpass(800, 0.7)`:
///   pre  = Some(VoiceRef("pluck"))
///   args = [Number(261.63), Number(329.63), Number(392.0), Number(4.0)]
///   post = Some(FnCall("lowpass", [2000, 0.7]))
pub fn extract_arp(expr: &Expr) -> Option<(Option<Expr>, Vec<Expr>, Option<Expr>)> {
    let segments = flatten_pipe_chain(expr);
    let arp_idx = segments.iter().position(
        |s| matches!(s, Expr::FnCall { name, .. } if name == "arp"),
    )?;

    let arp_args = match &segments[arp_idx] {
        Expr::FnCall { args, .. } => args.clone(),
        _ => unreachable!(),
    };

    let pre = if arp_idx > 0 {
        build_pipe_chain(&segments[..arp_idx])
    } else {
        None
    };

    let post = if arp_idx + 1 < segments.len() {
        build_pipe_chain(&segments[arp_idx + 1..])
    } else {
        None
    };

    Some((pre, arp_args, post))
}

const OSCILLATOR_NAMES: &[&str] = &["sine", "saw", "triangle", "square", "pulse"];

/// Walk an expression tree and replace every oscillator's frequency argument
/// with the given frequency. Resolves VoiceRefs by inlining the voice expression.
/// This lets `pluck >> arp(C4, E4, G4, 4)` substitute C4/E4/G4 into pluck's oscillators.
pub fn substitute_freq(expr: &Expr, voices: &HashMap<String, Expr>, wavetable_names: &[String], freq: f64) -> Expr {
    match expr {
        Expr::FnCall { name, args } if (OSCILLATOR_NAMES.contains(&name.as_str()) || wavetable_names.contains(name)) && !args.is_empty() => {
            let mut new_args = args.clone();
            new_args[0] = Expr::Number(freq);
            Expr::FnCall {
                name: name.clone(),
                args: new_args,
            }
        }
        Expr::VoiceRef(name) => {
            if let Some(voice_expr) = voices.get(name) {
                substitute_freq(voice_expr, voices, wavetable_names, freq)
            } else {
                expr.clone()
            }
        }
        Expr::Pipe(a, b) => Expr::Pipe(
            Box::new(substitute_freq(a, voices, wavetable_names, freq)),
            Box::new(substitute_freq(b, voices, wavetable_names, freq)),
        ),
        Expr::Sum(a, b) => Expr::Sum(
            Box::new(substitute_freq(a, voices, wavetable_names, freq)),
            Box::new(substitute_freq(b, voices, wavetable_names, freq)),
        ),
        Expr::Mul(a, b) => Expr::Mul(
            Box::new(substitute_freq(a, voices, wavetable_names, freq)),
            Box::new(substitute_freq(b, voices, wavetable_names, freq)),
        ),
        Expr::Div(a, b) => Expr::Div(
            Box::new(substitute_freq(a, voices, wavetable_names, freq)),
            Box::new(substitute_freq(b, voices, wavetable_names, freq)),
        ),
        Expr::Sub(a, b) => Expr::Sub(
            Box::new(substitute_freq(a, voices, wavetable_names, freq)),
            Box::new(substitute_freq(b, voices, wavetable_names, freq)),
        ),
        other => other.clone(),
    }
}

/// Walk an expression tree and replace all occurrences of a named variable
/// (VoiceRef matching `var`) with a numeric value. Unlike `substitute_freq` which
/// only replaces oscillator first-args, this replaces the variable everywhere —
/// including inside fn_call arguments like `lowpass(freq * 4, 0.7)`.
/// This is what makes instruments work: `freq` in the template gets replaced
/// with the actual Hz value at instantiation time.
pub fn substitute_var(expr: &Expr, var: &str, value: f64) -> Expr {
    match expr {
        Expr::VoiceRef(name) if name == var => Expr::Number(value),
        Expr::FnCall { name, args } => Expr::FnCall {
            name: name.clone(),
            args: args.iter().map(|a| substitute_var(a, var, value)).collect(),
        },
        Expr::Pipe(a, b) => Expr::Pipe(
            Box::new(substitute_var(a, var, value)),
            Box::new(substitute_var(b, var, value)),
        ),
        Expr::Sum(a, b) => {
            let sa = substitute_var(a, var, value);
            let sb = substitute_var(b, var, value);
            // Constant fold: Number + Number → Number
            if let (Expr::Number(a_val), Expr::Number(b_val)) = (&sa, &sb) {
                Expr::Number(a_val + b_val)
            } else {
                Expr::Sum(Box::new(sa), Box::new(sb))
            }
        }
        Expr::Mul(a, b) => {
            let sa = substitute_var(a, var, value);
            let sb = substitute_var(b, var, value);
            // Constant fold: Number * Number → Number
            if let (Expr::Number(a_val), Expr::Number(b_val)) = (&sa, &sb) {
                Expr::Number(a_val * b_val)
            } else {
                Expr::Mul(Box::new(sa), Box::new(sb))
            }
        }
        Expr::Div(a, b) => {
            let sa = substitute_var(a, var, value);
            let sb = substitute_var(b, var, value);
            // Constant fold: Number / Number → Number (with zero protection)
            if let (Expr::Number(a_val), Expr::Number(b_val)) = (&sa, &sb) {
                if *b_val == 0.0 {
                    Expr::Number(0.0) // division by zero → 0 (safe default)
                } else {
                    Expr::Number(a_val / b_val)
                }
            } else {
                Expr::Div(Box::new(sa), Box::new(sb))
            }
        }
        Expr::Sub(a, b) => {
            let sa = substitute_var(a, var, value);
            let sb = substitute_var(b, var, value);
            if let (Expr::Number(a_val), Expr::Number(b_val)) = (&sa, &sb) {
                Expr::Number(a_val - b_val)
            } else {
                Expr::Sub(Box::new(sa), Box::new(sb))
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
