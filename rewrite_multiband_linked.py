import re

with open("src/engine/effects.rs", "r") as f:
    content = f.read()

# Find start
start_marker = "// ---------------------------------------------------------------------------\n// Multiband Compressor (3-band: low, mid, high)\n// ---------------------------------------------------------------------------"

# Find end (end of MultibandCompressor impl)
end_marker = "    }\n}\n\n// ---------------------------------------------------------------------------\n// Master Bus Support Types"

start_idx = content.find(start_marker)
end_idx = content.find(end_marker)

if start_idx == -1 or end_idx == -1:
    print("Markers not found!")
    exit(1)

new_code = """// ---------------------------------------------------------------------------
// Multiband Compressor (3-band: low, mid, high)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct CrossoverLR4 {
    lp_b0: f32, lp_b1: f32, lp_b2: f32, lp_a1: f32, lp_a2: f32,
    hp_b0: f32, hp_b1: f32, hp_b2: f32, hp_a1: f32, hp_a2: f32,
    lp_state: [[[f32; 4]; 2]; 2], // [cascade][ch][state]
    hp_state: [[[f32; 4]; 2]; 2],
}

impl CrossoverLR4 {
    fn new(freq: f64, sr: f64) -> Self {
        let (lp_b0, lp_b1, lp_b2, lp_a1, lp_a2) = MultibandCompressor::lowpass_coeffs(freq, sr);
        let (hp_b0, hp_b1, hp_b2, hp_a1, hp_a2) = MultibandCompressor::highpass_coeffs(freq, sr);
        Self {
            lp_b0, lp_b1, lp_b2, lp_a1, lp_a2,
            hp_b0, hp_b1, hp_b2, hp_a1, hp_a2,
            lp_state: [[[0.0; 4]; 2]; 2],
            hp_state: [[[0.0; 4]; 2]; 2],
        }
    }
    
    #[inline]
    fn process_sample(&mut self, l: f32, r: f32) -> ((f32, f32), (f32, f32)) {
        let lp1_l = MultibandCompressor::biquad(l, &mut self.lp_state[0][0], self.lp_b0, self.lp_b1, self.lp_b2, self.lp_a1, self.lp_a2);
        let lp1_r = MultibandCompressor::biquad(r, &mut self.lp_state[0][1], self.lp_b0, self.lp_b1, self.lp_b2, self.lp_a1, self.lp_a2);
        let lp2_l = MultibandCompressor::biquad(lp1_l, &mut self.lp_state[1][0], self.lp_b0, self.lp_b1, self.lp_b2, self.lp_a1, self.lp_a2);
        let lp2_r = MultibandCompressor::biquad(lp1_r, &mut self.lp_state[1][1], self.lp_b0, self.lp_b1, self.lp_b2, self.lp_a1, self.lp_a2);
        
        let hp1_l = MultibandCompressor::biquad(l, &mut self.hp_state[0][0], self.hp_b0, self.hp_b1, self.hp_b2, self.hp_a1, self.hp_a2);
        let hp1_r = MultibandCompressor::biquad(r, &mut self.hp_state[0][1], self.hp_b0, self.hp_b1, self.hp_b2, self.hp_a1, self.hp_a2);
        let hp2_l = MultibandCompressor::biquad(hp1_l, &mut self.hp_state[1][0], self.hp_b0, self.hp_b1, self.hp_b2, self.hp_a1, self.hp_a2);
        let hp2_r = MultibandCompressor::biquad(hp1_r, &mut self.hp_state[1][1], self.hp_b0, self.hp_b1, self.hp_b2, self.hp_a1, self.hp_a2);
        
        ((lp2_l, lp2_r), (hp2_l, hp2_r))
    }
}

/// 3-band multiband compressor for the master bus.
/// Splits signal into low (<200Hz), mid (200Hz-3kHz), and high (>3kHz) bands
/// using 4th-order Linkwitz-Riley crossovers for perfect phase-aligned reconstruction.
#[derive(Clone)]
pub struct MultibandCompressor {
    lm_cross: CrossoverLR4,
    mh_cross: CrossoverLR4,
    low_align_cross: CrossoverLR4, // Phase aligns the low band with the mid/high crossover

    // Per-band compressor envelopes [low, mid, high]
    env: [f32; 3],
    // Per-band settings
    threshold: [f32; 3],
    ratio: [f32; 3],
    attack_coeff: [f32; 3],
    release_coeff: [f32; 3],
    makeup: [f32; 3],
    active: bool,
}

impl MultibandCompressor {
    pub fn new(sample_rate: f64) -> Self {
        // Attack/Release times must be frequency-dependent to avoid distortion!
        // Low: 15ms / 150ms (avoid tracking individual bass waves)
        // Mid: 5ms / 100ms
        // High: 1ms / 50ms
        let attacks = [0.015, 0.005, 0.001];
        let releases = [0.150, 0.100, 0.050];
        let mut attack_coeff = [0.0; 3];
        let mut release_coeff = [0.0; 3];
        
        for i in 0..3 {
            attack_coeff[i] = (-1.0 / (attacks[i] * sample_rate)).exp() as f32;
            release_coeff[i] = (-1.0 / (releases[i] * sample_rate)).exp() as f32;
        }

        MultibandCompressor {
            lm_cross: CrossoverLR4::new(200.0, sample_rate),
            mh_cross: CrossoverLR4::new(3000.0, sample_rate),
            low_align_cross: CrossoverLR4::new(3000.0, sample_rate),
            env: [0.0; 3],
            threshold: [-24.0, -20.0, -18.0],
            ratio: [3.0, 2.5, 2.0],
            attack_coeff,
            release_coeff,
            makeup: [1.0, 1.0, 1.0],
            active: false,
        }
    }

    /// Set from a simple amount (0 = off, 0.3 = gentle, 1.0 = heavy/OTT-level).
    pub fn set_amount(&mut self, amount: f32) {
        if amount <= 0.0 {
            self.active = false;
            return;
        }
        self.active = true;
        let a = amount.min(2.0);
        // Per-band thresholds: band signals are ~10-15 dB quieter than full mix.
        // Interpolate between gentle (0.3) and heavy (1.0) settings.
        let t = ((a - 0.3) / 0.7).clamp(0.0, 1.0);
        self.threshold = [
            -42.0 + t * ( -36.0 - -42.0),  // low:  -42 at 0.3, -36 at 1.0
            -38.0 + t * ( -32.0 - -38.0),  // mid:  -38 at 0.3, -32 at 1.0
            -34.0 + t * ( -28.0 - -34.0),  // high: -34 at 0.3, -28 at 1.0
        ];
        self.ratio = [
            1.9 + t * (4.0 - 1.9),   // low:  1.9 at 0.3, 4.0 at 1.0
            1.75 + t * (3.5 - 1.75),  // mid:  1.75 at 0.3, 3.5 at 1.0
            1.6 + t * (3.0 - 1.6),    // high: 1.6 at 0.3, 3.0 at 1.0
        ];
        // Conservative per-band makeup to partially restore loudness.
        // Keep it modest — the following chain stages handle the rest.
        for i in 0..3 {
            let max_reduction = self.threshold[i].abs() * (1.0 - 1.0 / self.ratio[i]);
            let makeup_db = (max_reduction * 0.1).min(4.0); // 10% of theoretical max, cap 4 dB
            self.makeup[i] = 10.0_f32.powf(makeup_db / 20.0);
        }
    }

    /// Set per-band amounts (low, mid, high). Each 0.0-2.0.
    pub fn set_per_band(&mut self, low: f32, mid: f32, high: f32) {
        self.active = low > 0.0 || mid > 0.0 || high > 0.0;
        let amounts = [low, mid, high];
        let base_thresholds_gentle = [-42.0, -38.0, -34.0];
        let base_thresholds_heavy = [-36.0, -32.0, -28.0];
        let ratios_gentle = [1.9, 1.75, 1.6];
        let ratios_heavy = [4.0, 3.5, 3.0];
        for i in 0..3 {
            let a = amounts[i].max(0.0).min(2.0);
            if a <= 0.0 {
                self.threshold[i] = -100.0;
                self.ratio[i] = 1.0;
                self.makeup[i] = 1.0;
            } else {
                let t = ((a - 0.3) / 0.7).clamp(0.0, 1.0);
                self.threshold[i] = base_thresholds_gentle[i] + t * (base_thresholds_heavy[i] - base_thresholds_gentle[i]);
                self.ratio[i] = ratios_gentle[i] + t * (ratios_heavy[i] - ratios_gentle[i]);
                let max_reduction = self.threshold[i].abs() * (1.0 - 1.0 / self.ratio[i]);
                let makeup_db = (max_reduction * 0.1).min(4.0);
                self.makeup[i] = 10.0_f32.powf(makeup_db / 20.0);
            }
        }
    }

    fn lowpass_coeffs(freq: f64, sr: f64) -> (f32, f32, f32, f32, f32) {
        let w0 = 2.0 * std::f64::consts::PI * freq / sr;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * std::f64::consts::SQRT_2);
        let a0 = 1.0 + alpha;
        ((((1.0 - cos_w0) / 2.0) / a0) as f32,
         (((1.0 - cos_w0)) / a0) as f32,
         ((((1.0 - cos_w0) / 2.0)) / a0) as f32,
         ((-2.0 * cos_w0) / a0) as f32,
         (((1.0 - alpha)) / a0) as f32)
    }

    fn highpass_coeffs(freq: f64, sr: f64) -> (f32, f32, f32, f32, f32) {
        let w0 = 2.0 * std::f64::consts::PI * freq / sr;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * std::f64::consts::SQRT_2);
        let a0 = 1.0 + alpha;
        ((((1.0 + cos_w0) / 2.0) / a0) as f32,
         ((-(1.0 + cos_w0)) / a0) as f32,
         ((((1.0 + cos_w0) / 2.0)) / a0) as f32,
         ((-2.0 * cos_w0) / a0) as f32,
         (((1.0 - alpha)) / a0) as f32)
    }

    #[inline]
    fn biquad(x: f32, state: &mut [f32; 4], b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) -> f32 {
        let y = b0 * x + b1 * state[0] + b2 * state[1] - a1 * state[2] - a2 * state[3];
        state[1] = state[0]; state[0] = x;
        state[3] = state[2]; state[2] = y;
        y
    }

    #[inline]
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
        let (low_lp, low_hp) = self.low_align_cross.process_sample(low_unaligned.0, low_unaligned.1);
        let low = (low_lp.0 + low_hp.0, low_lp.1 + low_hp.1);

        // Compress each band independently, with stereo-linked gain
        let (comp_low_l, comp_low_r) = self.compress_stereo_sample(low.0, low.1, 0);
        let (comp_mid_l, comp_mid_r) = self.compress_stereo_sample(mid.0, mid.1, 1);
        let (comp_high_l, comp_high_r) = self.compress_stereo_sample(high.0, high.1, 2);

        // Sum compressed bands directly
        (comp_low_l + comp_mid_l + comp_high_l,
         comp_low_r + comp_mid_r + comp_high_r)
    }
}
"""

with open("src/engine/effects.rs", "w") as f:
    f.write(content[:start_idx] + new_code + content[end_idx:])

print("Replacement successful!")
