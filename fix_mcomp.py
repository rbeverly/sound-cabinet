import sys

with open("src/engine/effects.rs", "r") as f:
    content = f.read()

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
                    let x_knee = self.threshold + knee / 2.0 - env_db;
                    let boosted = (1.0 - 1.0 / self.ratio) * x_knee * x_knee / (2.0 * knee);
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
                    let x_knee = env_db - self.threshold + knee / 2.0;
                    -(1.0 - 1.0 / self.ratio) * x_knee * x_knee / (2.0 * knee)
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

with open("src/engine/effects.rs", "w") as f:
    f.write(content)

print("effects.rs replaced successfully")

