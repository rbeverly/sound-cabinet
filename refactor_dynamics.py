import re
import sys

with open("src/engine/effects.rs", "r") as f:
    content = f.read()

# 1. Update BrickwallLimiter
old_limiter = """pub struct BrickwallLimiter {
    ceiling: f32,
    release_coeff: f32,
    gain_reduction: f32, // current gain reduction (0.0 = no reduction, higher = more)
    lookahead_buf: Vec<f32>,
    lookahead_pos: usize,
    lookahead_len: usize,
}

impl BrickwallLimiter {
    pub fn new(ceiling: f32, release_secs: f64, sample_rate: f64) -> Self {
        // 5ms lookahead — enough to catch transients cleanly
        let lookahead_len = (0.005 * sample_rate) as usize;
        let release_coeff = (-1.0 / (release_secs * sample_rate)).exp() as f32;
        BrickwallLimiter {
            ceiling,
            release_coeff,
            gain_reduction: 0.0,
            lookahead_buf: vec![0.0; lookahead_len],
            lookahead_pos: 0,
            lookahead_len,
        }
    }

    /// Process a buffer of samples in-place.
    pub fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            // Write current sample into lookahead buffer, read delayed sample
            let delayed = self.lookahead_buf[self.lookahead_pos];
            self.lookahead_buf[self.lookahead_pos] = *sample;
            self.lookahead_pos = (self.lookahead_pos + 1) % self.lookahead_len;

            // Compute required gain reduction from current (pre-delay) sample
            let abs_val = sample.abs();
            let needed = if abs_val > self.ceiling {
                1.0 - self.ceiling / abs_val
            } else {
                0.0
            };

            // Attack is instant (lookahead handles smoothing), release is gradual
            if needed > self.gain_reduction {
                self.gain_reduction = needed;
            } else {
                self.gain_reduction =
                    self.release_coeff * self.gain_reduction + (1.0 - self.release_coeff) * needed;
            }

            // Apply gain to the delayed sample
            let gain = 1.0 - self.gain_reduction;
            *sample = delayed * gain;
        }
    }

    /// Flush the lookahead buffer (call after all audio is processed).
    pub fn flush(&mut self, output: &mut Vec<f32>) {
        for _ in 0..self.lookahead_len {
            let delayed = self.lookahead_buf[self.lookahead_pos];
            self.lookahead_buf[self.lookahead_pos] = 0.0;
            self.lookahead_pos = (self.lookahead_pos + 1) % self.lookahead_len;

            let gain = 1.0 - self.gain_reduction;
            self.gain_reduction =
                self.release_coeff * self.gain_reduction;
            output.push(delayed * gain);
        }
    }
}"""

new_limiter = """pub struct BrickwallLimiter {
    ceiling: f32,
    release_coeff: f32,
    gain_reduction: f32, // current gain reduction (0.0 = no reduction, higher = more)
    lookahead_buf_l: Vec<f32>,
    lookahead_buf_r: Vec<f32>,
    lookahead_pos: usize,
    lookahead_len: usize,
    peak_hold_timer: usize,
}

impl BrickwallLimiter {
    pub fn new(ceiling: f32, release_secs: f64, sample_rate: f64) -> Self {
        // 5ms lookahead — enough to catch transients cleanly
        let lookahead_len = (0.005 * sample_rate) as usize;
        let release_coeff = (-1.0 / (release_secs * sample_rate)).exp() as f32;
        BrickwallLimiter {
            ceiling,
            release_coeff,
            gain_reduction: 0.0,
            lookahead_buf_l: vec![0.0; lookahead_len],
            lookahead_buf_r: vec![0.0; lookahead_len],
            lookahead_pos: 0,
            lookahead_len,
            peak_hold_timer: 0,
        }
    }

    /// Process a stereo buffer of samples in-place.
    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let delayed_l = self.lookahead_buf_l[self.lookahead_pos];
            let delayed_r = self.lookahead_buf_r[self.lookahead_pos];
            self.lookahead_buf_l[self.lookahead_pos] = *l;
            self.lookahead_buf_r[self.lookahead_pos] = *r;
            self.lookahead_pos = (self.lookahead_pos + 1) % self.lookahead_len;

            // Compute required gain reduction from current (pre-delay) sample
            // Stereo linking: use the maximum absolute value of both channels
            let max_abs = l.abs().max(r.abs());
            let needed = if max_abs > self.ceiling {
                1.0 - self.ceiling / max_abs
            } else {
                0.0
            };

            // Attack is instant (lookahead handles smoothing), release is gradual
            if needed > self.gain_reduction {
                self.gain_reduction = needed;
                self.peak_hold_timer = self.lookahead_len; // Hold for lookahead window
            } else if self.peak_hold_timer > 0 {
                self.peak_hold_timer -= 1;
            } else {
                self.gain_reduction =
                    self.release_coeff * self.gain_reduction + (1.0 - self.release_coeff) * needed;
            }

            // Apply gain to the delayed sample
            let gain = 1.0 - self.gain_reduction;
            *l = delayed_l * gain;
            *r = delayed_r * gain;
        }
    }

    /// Flush the lookahead buffer (call after all audio is processed).
    pub fn flush_stereo(&mut self, left: &mut Vec<f32>, right: &mut Vec<f32>) {
        for _ in 0..self.lookahead_len {
            let delayed_l = self.lookahead_buf_l[self.lookahead_pos];
            let delayed_r = self.lookahead_buf_r[self.lookahead_pos];
            self.lookahead_buf_l[self.lookahead_pos] = 0.0;
            self.lookahead_buf_r[self.lookahead_pos] = 0.0;
            self.lookahead_pos = (self.lookahead_pos + 1) % self.lookahead_len;

            let gain = 1.0 - self.gain_reduction;
            if self.peak_hold_timer > 0 {
                self.peak_hold_timer -= 1;
            } else {
                self.gain_reduction = self.release_coeff * self.gain_reduction;
            }
            left.push(delayed_l * gain);
            right.push(delayed_r * gain);
        }
    }
}"""

if old_limiter not in content:
    print("Failed to find BrickwallLimiter")
    sys.exit(1)
content = content.replace(old_limiter, new_limiter)


# 2. Update MasterCompressor process -> process_stereo
old_mcomp_proc = """    /// Process a buffer in-place.
    pub fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            let x = *sample;

            // Guard: NaN/Inf would poison envelope state permanently
            if !x.is_finite() {
                *sample = 0.0;
                continue;
            }

            let x_sq = x * x;

            // RMS envelope follower (track squared signal, compare in dB domain)
            if x_sq > self.envelope_sq {
                self.envelope_sq = self.attack_coeff * self.envelope_sq
                    + (1.0 - self.attack_coeff) * x_sq;
            } else {
                self.envelope_sq = self.release_coeff * self.envelope_sq
                    + (1.0 - self.release_coeff) * x_sq;
            }

            // Safety: if envelope went bad, reset it
            if !self.envelope_sq.is_finite() {
                self.envelope_sq = 0.0;
                *sample = 0.0;
                continue;
            }

            // Convert RMS to dB (RMS = sqrt(envelope_sq), dB = 20*log10(rms))
            // = 10*log10(envelope_sq)
            let env_db = if self.envelope_sq > 1e-20 {
                10.0 * self.envelope_sq.log10()
            } else {
                -200.0
            };

            // Gain change: downward or upward compression with soft knee
            // (Giannoulis/Massberg/Reiss JAES 2012)
            let knee = self.knee_width;
            let gain_db = if self.upward {
                // Upward: boost content BELOW threshold with soft knee
                if env_db > self.threshold + knee / 2.0 || env_db < -100.0 {
                    // Above knee or silence: no boost
                    0.0
                } else if env_db < self.threshold - knee / 2.0 {
                    // Below knee: full upward compression
                    let under = self.threshold - env_db;
                    let boosted = under * (1.0 - 1.0 / self.ratio);
                    boosted.min(24.0) // cap boost at 24 dB to prevent runaway
                } else {
                    // In knee zone: smooth transition
                    let x = self.threshold + knee / 2.0 - env_db;
                    let boosted = (1.0 - 1.0 / self.ratio) * x * x / (2.0 * knee);
                    boosted.min(24.0)
                }
            } else {
                // Downward: reduce content ABOVE threshold with soft knee
                if env_db < self.threshold - knee / 2.0 {
                    // Below knee: no compression
                    0.0
                } else if env_db > self.threshold + knee / 2.0 {
                    // Above knee: full compression
                    let over = env_db - self.threshold;
                    -(over * (1.0 - 1.0 / self.ratio))
                } else {
                    // In knee zone: smooth transition
                    let x = env_db - self.threshold + knee / 2.0;
                    -(1.0 - 1.0 / self.ratio) * x * x / (2.0 * knee)
                }
            };

            let gain = 10.0_f32.powf(gain_db / 20.0) * self.makeup_gain;
            *sample = x * gain;
        }
    }"""

new_mcomp_proc = """    /// Process a stereo buffer in-place, stereo-linked.
    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let max_abs = l.abs().max(r.abs());

            // Guard: NaN/Inf would poison envelope state permanently
            if !max_abs.is_finite() {
                *l = 0.0;
                *r = 0.0;
                continue;
            }

            let max_sq = max_abs * max_abs;

            // RMS envelope follower (track squared signal, compare in dB domain)
            if max_sq > self.envelope_sq {
                self.envelope_sq = self.attack_coeff * self.envelope_sq
                    + (1.0 - self.attack_coeff) * max_sq;
            } else {
                self.envelope_sq = self.release_coeff * self.envelope_sq
                    + (1.0 - self.release_coeff) * max_sq;
            }

            // Safety: if envelope went bad, reset it
            if !self.envelope_sq.is_finite() {
                self.envelope_sq = 0.0;
                *l = 0.0;
                *r = 0.0;
                continue;
            }

            // Convert RMS to dB (RMS = sqrt(envelope_sq), dB = 20*log10(rms))
            // = 10*log10(envelope_sq)
            let env_db = if self.envelope_sq > 1e-20 {
                10.0 * self.envelope_sq.log10()
            } else {
                -200.0
            };

            let knee = self.knee_width;
            let gain_db = if self.upward {
                if env_db > self.threshold + knee / 2.0 || env_db < -100.0 {
                    0.0
                } else if env_db < self.threshold - knee / 2.0 {
                    let under = self.threshold - env_db;
                    let boosted = under * (1.0 - 1.0 / self.ratio);
                    boosted.min(24.0)
                } else {
                    let x_knee = self.threshold + knee / 2.0 - env_db;
                    let boosted = (1.0 - 1.0 / self.ratio) * x_knee * x_knee / (2.0 * knee);
                    boosted.min(24.0)
                }
            } else {
                if env_db < self.threshold - knee / 2.0 {
                    0.0
                } else if env_db > self.threshold + knee / 2.0 {
                    let over = env_db - self.threshold;
                    -(over * (1.0 - 1.0 / self.ratio))
                } else {
                    let x_knee = env_db - self.threshold + knee / 2.0;
                    -(1.0 - 1.0 / self.ratio) * x_knee * x_knee / (2.0 * knee)
                }
            };

            let gain = 10.0_f32.powf(gain_db / 20.0) * self.makeup_gain;
            *l *= gain;
            *r *= gain;
        }
    }"""

if old_mcomp_proc not in content:
    print("Failed to find MasterCompressor process")
    sys.exit(1)
content = content.replace(old_mcomp_proc, new_mcomp_proc)


# 3. MultibandCompressor process -> linked
old_mb_env1 = """    // Per-band compressor envelopes [low, mid, high] × [left, right]
    env: [[f32; 2]; 3],"""
new_mb_env1 = """    // Per-band compressor envelopes [low, mid, high]
    env: [f32; 3],"""
content = content.replace(old_mb_env1, new_mb_env1)

old_mb_env2 = """            env: [[0.0; 2]; 3],"""
new_mb_env2 = """            env: [0.0; 3],"""
content = content.replace(old_mb_env2, new_mb_env2)

old_mb_compress_sample = """    #[inline]
    fn compress_sample(&mut self, sample: f32, band: usize, ch: usize) -> f32 {
        let abs = sample.abs();
        // Peak envelope follower
        if abs > self.env[band][ch] {
            self.env[band][ch] = self.attack_coeff[band] * self.env[band][ch] + (1.0 - self.attack_coeff[band]) * abs;
        } else {
            self.env[band][ch] = self.release_coeff[band] * self.env[band][ch] + (1.0 - self.release_coeff[band]) * abs;
        }

        let env = self.env[band][ch];
        if env < 1e-10 { return sample; }

        let env_db = 20.0 * env.log10();

        // Downward compression with soft knee (6 dB knee width)
        let knee: f32 = 6.0;
        let gain_db = if env_db < self.threshold[band] - knee / 2.0 {
            // Below knee: no compression
            0.0
        } else if env_db > self.threshold[band] + knee / 2.0 {
            // Above knee: full compression
            let over = env_db - self.threshold[band];
            -(over * (1.0 - 1.0 / self.ratio[band]))
        } else {
            // In knee zone: smooth transition
            let x = env_db - self.threshold[band] + knee / 2.0;
            -(1.0 - 1.0 / self.ratio[band]) * x * x / (2.0 * knee)
        };

        sample * 10.0_f32.powf(gain_db / 20.0) * self.makeup[band]
    }

    /// Process a stereo sample pair through the multiband compressor.
    /// Returns (left_out, right_out).
    ///
    /// Uses 4th-order Linkwitz-Riley crossovers for perfect phase-aligned reconstruction.
    #[inline]
    pub fn process_sample(&mut self, l: f32, r: f32) -> (f32, f32) {
        if !self.active { return (l, r); }

        // Low/Mid-High Split
        let (low_unaligned, mid_high) = self.lm_cross.process_sample(l, r);
        
        // Mid/High Split
        let (mid, high) = self.mh_cross.process_sample(mid_high.0, mid_high.1);
        
        // Phase align the low band with the mid/high crossover delay
        // An LR4 crossover acts as an allpass filter to the sum of its outputs
        let (low_lp, low_hp) = self.low_align_cross.process_sample(low_unaligned.0, low_unaligned.1);
        let low = (low_lp.0 + low_hp.0, low_lp.1 + low_hp.1);

        // Compress each band independently
        let comp_low_l = self.compress_sample(low.0, 0, 0);
        let comp_low_r = self.compress_sample(low.1, 0, 1);
        let comp_mid_l = self.compress_sample(mid.0, 1, 0);
        let comp_mid_r = self.compress_sample(mid.1, 1, 1);
        let comp_high_l = self.compress_sample(high.0, 2, 0);
        let comp_high_r = self.compress_sample(high.1, 2, 1);

        // Sum compressed bands directly (perfect reconstruction when makeup is 1.0 and no compression)
        (comp_low_l + comp_mid_l + comp_high_l,
         comp_low_r + comp_mid_r + comp_high_r)
    }"""

new_mb_compress_sample = """    #[inline]
    fn compress_stereo_sample(&mut self, l: f32, r: f32, band: usize) -> (f32, f32) {
        let max_abs = l.abs().max(r.abs());
        
        // Peak envelope follower
        if max_abs > self.env[band] {
            self.env[band] = self.attack_coeff[band] * self.env[band] + (1.0 - self.attack_coeff[band]) * max_abs;
        } else {
            self.env[band] = self.release_coeff[band] * self.env[band] + (1.0 - self.release_coeff[band]) * max_abs;
        }

        let env = self.env[band];
        if env < 1e-10 { return (l, r); }

        let env_db = 20.0 * env.log10();

        // Downward compression with soft knee (6 dB knee width)
        let knee: f32 = 6.0;
        let gain_db = if env_db < self.threshold[band] - knee / 2.0 {
            0.0
        } else if env_db > self.threshold[band] + knee / 2.0 {
            let over = env_db - self.threshold[band];
            -(over * (1.0 - 1.0 / self.ratio[band]))
        } else {
            let x = env_db - self.threshold[band] + knee / 2.0;
            -(1.0 - 1.0 / self.ratio[band]) * x * x / (2.0 * knee)
        };

        let gain = 10.0_f32.powf(gain_db / 20.0) * self.makeup[band];
        (l * gain, r * gain)
    }

    /// Process a stereo sample pair through the multiband compressor.
    /// Returns (left_out, right_out).
    ///
    /// Uses 4th-order Linkwitz-Riley crossovers for perfect phase-aligned reconstruction.
    #[inline]
    pub fn process_sample(&mut self, l: f32, r: f32) -> (f32, f32) {
        if !self.active { return (l, r); }

        // Low/Mid-High Split
        let (low_unaligned, mid_high) = self.lm_cross.process_sample(l, r);
        
        // Mid/High Split
        let (mid, high) = self.mh_cross.process_sample(mid_high.0, mid_high.1);
        
        // Phase align the low band with the mid/high crossover delay
        // An LR4 crossover acts as an allpass filter to the sum of its outputs
        let (low_lp, low_hp) = self.low_align_cross.process_sample(low_unaligned.0, low_unaligned.1);
        let low = (low_lp.0 + low_hp.0, low_lp.1 + low_hp.1);

        // Compress each band independently, with stereo-linked gain
        let (comp_low_l, comp_low_r) = self.compress_stereo_sample(low.0, low.1, 0);
        let (comp_mid_l, comp_mid_r) = self.compress_stereo_sample(mid.0, mid.1, 1);
        let (comp_high_l, comp_high_r) = self.compress_stereo_sample(high.0, high.1, 2);

        // Sum compressed bands directly (perfect reconstruction when makeup is 1.0 and no compression)
        (comp_low_l + comp_mid_l + comp_high_l,
         comp_low_r + comp_mid_r + comp_high_r)
    }"""

if old_mb_compress_sample not in content:
    print("Failed to find MultibandCompressor process_sample")
    sys.exit(1)
content = content.replace(old_mb_compress_sample, new_mb_compress_sample)


# 4. Update StageCompress
old_stage_comp = """pub struct StageCompress {
    pub compressor: [MasterCompressor; 2],
}

impl StageCompress {
    pub fn new(amount: f32, sample_rate: f64) -> Self {
        let c = MasterCompressor::from_amount(amount, sample_rate);
        StageCompress { compressor: [c.clone(), c] }
    }

    pub fn from_params(threshold: f32, ratio: f32, attack: f64, release: f64, sample_rate: f64) -> Self {
        let c = MasterCompressor::new(threshold, ratio, attack, release, sample_rate);
        StageCompress { compressor: [c.clone(), c] }
    }
}

impl MasterStage for StageCompress {
    fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.compressor[0].process(left);
        self.compressor[1].process(right);
    }
    fn name(&self) -> &'static str { "compress" }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}"""

new_stage_comp = """pub struct StageCompress {
    pub compressor: MasterCompressor,
}

impl StageCompress {
    pub fn new(amount: f32, sample_rate: f64) -> Self {
        StageCompress { compressor: MasterCompressor::from_amount(amount, sample_rate) }
    }

    pub fn from_params(threshold: f32, ratio: f32, attack: f64, release: f64, sample_rate: f64) -> Self {
        StageCompress { compressor: MasterCompressor::new(threshold, ratio, attack, release, sample_rate) }
    }
}

impl MasterStage for StageCompress {
    fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.compressor.process_stereo(left, right);
    }
    fn name(&self) -> &'static str { "compress" }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}"""

if old_stage_comp not in content:
    print("Failed to find StageCompress")
    sys.exit(1)
content = content.replace(old_stage_comp, new_stage_comp)


# 5. Update StageExpand
old_stage_expand = """pub struct StageExpand {
    pub expander: [Expander; 2],
}

impl StageExpand {
    pub fn new(threshold_db: f32, ratio: f32, attack_secs: f64, release_secs: f64) -> Self {
        StageExpand {
            expander: [
                Expander::new(threshold_db, ratio, attack_secs, release_secs),
                Expander::new(threshold_db, ratio, attack_secs, release_secs),
            ],
        }
    }
}

impl MasterStage for StageExpand {
    fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        for sample in left.iter_mut() {
            let frame: fundsp::hacker::Frame<f32, fundsp::hacker::U1> = [*sample].into();
            let out = self.expander[0].tick(&frame);
            *sample = out[0];
        }
        for sample in right.iter_mut() {
            let frame: fundsp::hacker::Frame<f32, fundsp::hacker::U1> = [*sample].into();
            let out = self.expander[1].tick(&frame);
            *sample = out[0];
        }
    }
    fn name(&self) -> &'static str { "expand" }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}"""

new_stage_expand = """pub struct StageExpand {
    threshold: f32,
    ratio: f32,
    attack_coeff: f32,
    release_coeff: f32,
    envelope: f32,
}

impl StageExpand {
    pub fn new(threshold_db: f32, ratio: f32, attack_secs: f64, release_secs: f64, sample_rate: f64) -> Self {
        StageExpand {
            threshold: threshold_db,
            ratio: ratio.max(1.0),
            attack_coeff: (-1.0 / (attack_secs * sample_rate)).exp() as f32,
            release_coeff: (-1.0 / (release_secs * sample_rate)).exp() as f32,
            envelope: 0.0,
        }
    }
}

impl MasterStage for StageExpand {
    fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let max_abs = l.abs().max(r.abs());

            if max_abs > self.envelope {
                self.envelope = self.attack_coeff * self.envelope + (1.0 - self.attack_coeff) * max_abs;
            } else {
                self.envelope = self.release_coeff * self.envelope + (1.0 - self.release_coeff) * max_abs;
            }

            if self.envelope < 1e-10 { continue; }

            let env_db = 20.0 * self.envelope.log10();
            let knee: f32 = 6.0;

            let gain_db = if env_db > self.threshold + knee / 2.0 || env_db < -100.0 {
                0.0
            } else if env_db < self.threshold - knee / 2.0 {
                let under = self.threshold - env_db;
                -under * (1.0 - 1.0 / self.ratio)
            } else {
                let x = self.threshold + knee / 2.0 - env_db;
                -(1.0 - 1.0 / self.ratio) * x * x / (2.0 * knee)
            };

            let gain = 10.0_f32.powf(gain_db / 20.0);
            *l *= gain;
            *r *= gain;
        }
    }
    fn name(&self) -> &'static str { "expand" }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}"""

if old_stage_expand not in content:
    print("Failed to find StageExpand")
    sys.exit(1)
content = content.replace(old_stage_expand, new_stage_expand)


# 6. Update MasterBus limiter fields and methods
old_mb_limiter = """    // Limiter per channel [left, right]
    limiter: [BrickwallLimiter; 2],"""
new_mb_limiter = """    // Stereo-linked Brickwall Limiter
    limiter: BrickwallLimiter,"""
content = content.replace(old_mb_limiter, new_mb_limiter)

old_mb_new = """        let limiter = BrickwallLimiter::new(ceiling, 0.1, sample_rate);
        // Default chain: single compressor at amount 1.0
        let chain: Vec<Box<dyn MasterStage>> = vec![
            Box::new(StageCompress::new(1.0, sample_rate)),
        ];
        MasterBus {
            hp_a1, hp_a2, hp_b0, hp_b1, hp_b2,
            hp_state: [BiquadState::default(), BiquadState::default()],
            lp_a1, lp_a2, lp_b0, lp_b1, lp_b2,
            lp_state: [BiquadState::default(), BiquadState::default()],
            limiter: [limiter.clone(), limiter],
            gain: 1.0,
            chain,
        }"""
new_mb_new = """        let limiter = BrickwallLimiter::new(ceiling, 0.1, sample_rate);
        // Default chain: single compressor at amount 1.0
        let chain: Vec<Box<dyn MasterStage>> = vec![
            Box::new(StageCompress::new(1.0, sample_rate)),
        ];
        MasterBus {
            hp_a1, hp_a2, hp_b0, hp_b1, hp_b2,
            hp_state: [BiquadState::default(), BiquadState::default()],
            lp_a1, lp_a2, lp_b0, lp_b1, lp_b2,
            lp_state: [BiquadState::default(), BiquadState::default()],
            limiter,
            gain: 1.0,
            chain,
        }"""
content = content.replace(old_mb_new, new_mb_new)

old_mb_process1 = """        // Limiter
        self.limiter[0].process(buffer);"""
new_mb_process1 = """        // Limiter
        self.limiter.process_stereo(buffer, &mut right_dummy);"""
content = content.replace(old_mb_process1, new_mb_process1)

old_mb_process2 = """        // Limiter per channel
        self.limiter[0].process(left);
        self.limiter[1].process(right);"""
new_mb_process2 = """        // Limiter stereo
        self.limiter.process_stereo(left, right);"""
content = content.replace(old_mb_process2, new_mb_process2)

old_mb_flush = """    /// Flush limiter lookahead tail (mono — uses channel 0).
    pub fn flush(&mut self, output: &mut Vec<f32>) {
        self.limiter[0].flush(output);
    }

    /// Flush both limiter channels for stereo output.
    pub fn flush_stereo(&mut self, left: &mut Vec<f32>, right: &mut Vec<f32>) {
        self.limiter[0].flush(left);
        self.limiter[1].flush(right);
    }"""
new_mb_flush = """    /// Flush limiter lookahead tail (mono).
    pub fn flush(&mut self, output: &mut Vec<f32>) {
        let mut dummy = Vec::new();
        self.limiter.flush_stereo(output, &mut dummy);
    }

    /// Flush both limiter channels for stereo output.
    pub fn flush_stereo(&mut self, left: &mut Vec<f32>, right: &mut Vec<f32>) {
        self.limiter.flush_stereo(left, right);
    }"""
content = content.replace(old_mb_flush, new_mb_flush)

old_add_expand = """    pub fn add_expand(&mut self, threshold_db: f32, ratio: f32, attack: f64, release: f64) {
        self.replace_or_append("expand", Box::new(StageExpand::new(threshold_db, ratio, attack, release)));
    }"""
new_add_expand = """    pub fn add_expand(&mut self, threshold_db: f32, ratio: f32, attack: f64, release: f64) {
        self.replace_or_append("expand", Box::new(StageExpand::new(threshold_db, ratio, attack, release, self.sample_rate)));
    }"""
content = content.replace(old_add_expand, new_add_expand)

with open("src/engine/effects.rs", "w") as f:
    f.write(content)

print("effects.rs replaced successfully")

with open("src/engine/engine.rs", "r") as f:
    eng_content = f.read()

old_eng_expand = """                    Box::new(StageExpand::new(threshold, ratio, attack, release))"""
new_eng_expand = """                    Box::new(StageExpand::new(threshold, ratio, attack, release, sr))"""
eng_content = eng_content.replace(old_eng_expand, new_eng_expand)

with open("src/engine/engine.rs", "w") as f:
    f.write(eng_content)

print("engine.rs replaced successfully")
