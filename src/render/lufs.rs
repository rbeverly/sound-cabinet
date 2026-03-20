//! ITU-R BS.1770 integrated loudness measurement (mono).
//!
//! Implements the K-weighting filter and gated block integration
//! to produce an integrated LUFS value from a buffer of f32 samples.

/// Measure integrated loudness (LUFS) of a mono signal at the given sample rate.
///
/// Implements ITU-R BS.1770-4:
/// 1. K-weighting filter (high-shelf + highpass)
/// 2. 400ms block mean-square
/// 3. Absolute gate (-70 LUFS)
/// 4. Relative gate (-10 dB below ungated mean)
pub fn measure_lufs(samples: &[f32], sample_rate: f64) -> f64 {
    if samples.is_empty() {
        return -f64::INFINITY;
    }

    // Step 1: K-weighting filter
    let weighted = k_weight(samples, sample_rate);

    // Step 2: Compute mean-square per 400ms block (75% overlap → step = 100ms)
    let block_len = (0.4 * sample_rate) as usize;
    let step_len = (0.1 * sample_rate) as usize;

    if block_len == 0 || weighted.len() < block_len {
        // Signal too short for even one block — just compute overall
        let ms: f64 = weighted.iter().map(|&s| (s as f64) * (s as f64)).sum::<f64>()
            / weighted.len() as f64;
        return -0.691 + 10.0 * ms.log10();
    }

    let mut block_powers: Vec<f64> = Vec::new();
    let mut pos = 0;
    while pos + block_len <= weighted.len() {
        let ms: f64 = weighted[pos..pos + block_len]
            .iter()
            .map(|&s| (s as f64) * (s as f64))
            .sum::<f64>()
            / block_len as f64;
        block_powers.push(ms);
        pos += step_len;
    }

    if block_powers.is_empty() {
        return -f64::INFINITY;
    }

    // Step 3: Absolute gate at -70 LUFS
    let abs_gate_threshold = 10.0_f64.powf((-70.0 + 0.691) / 10.0);
    let ungated: Vec<f64> = block_powers
        .iter()
        .copied()
        .filter(|&p| p > abs_gate_threshold)
        .collect();

    if ungated.is_empty() {
        return -f64::INFINITY;
    }

    let ungated_mean = ungated.iter().sum::<f64>() / ungated.len() as f64;
    let ungated_lufs = -0.691 + 10.0 * ungated_mean.log10();

    // Step 4: Relative gate at -10 dB below ungated mean
    let rel_gate_threshold = 10.0_f64.powf((ungated_lufs - 10.0 + 0.691) / 10.0);
    let gated: Vec<f64> = block_powers
        .iter()
        .copied()
        .filter(|&p| p > rel_gate_threshold)
        .collect();

    if gated.is_empty() {
        return -f64::INFINITY;
    }

    let gated_mean = gated.iter().sum::<f64>() / gated.len() as f64;
    -0.691 + 10.0 * gated_mean.log10()
}

/// Find the true peak in dBFS.
pub fn true_peak_dbfs(samples: &[f32]) -> f64 {
    let peak = samples
        .iter()
        .map(|s| s.abs())
        .fold(0.0_f32, f32::max);
    if peak > 0.0 {
        20.0 * (peak as f64).log10()
    } else {
        -f64::INFINITY
    }
}

/// Apply K-weighting filter (ITU-R BS.1770) to mono samples.
///
/// Two cascaded biquad stages:
/// 1. High-shelf boost (~+4 dB above 1.5 kHz) — models head diffraction
/// 2. Highpass at ~38 Hz — removes DC and sub-bass from measurement
fn k_weight(samples: &[f32], sr: f64) -> Vec<f32> {
    // Stage 1: High-shelf filter coefficients (from BS.1770-4)
    let (s1_b0, s1_b1, s1_b2, s1_a1, s1_a2) = k_weight_shelf(sr);
    // Stage 2: Highpass filter coefficients
    let (s2_b0, s2_b1, s2_b2, s2_a1, s2_a2) = k_weight_highpass(sr);

    let mut out = vec![0.0f32; samples.len()];

    // Stage 1 state
    let (mut x1_1, mut x2_1, mut y1_1, mut y2_1) = (0.0f64, 0.0, 0.0, 0.0);
    // Stage 2 state
    let (mut x1_2, mut x2_2, mut y1_2, mut y2_2) = (0.0f64, 0.0, 0.0, 0.0);

    for (i, &s) in samples.iter().enumerate() {
        let x = s as f64;

        // Stage 1: shelf
        let y = s1_b0 * x + s1_b1 * x1_1 + s1_b2 * x2_1 - s1_a1 * y1_1 - s1_a2 * y2_1;
        x2_1 = x1_1;
        x1_1 = x;
        y2_1 = y1_1;
        y1_1 = y;

        // Stage 2: highpass
        let z = s2_b0 * y + s2_b1 * x1_2 + s2_b2 * x2_2 - s2_a1 * y1_2 - s2_a2 * y2_2;
        x2_2 = x1_2;
        x1_2 = y;
        y2_2 = y1_2;
        y1_2 = z;

        out[i] = z as f32;
    }

    out
}

/// K-weighting stage 1: high-shelf boost.
/// Coefficients from ITU-R BS.1770-4, Table 1 (for 48kHz reference, bilinear-transformed).
fn k_weight_shelf(sr: f64) -> (f64, f64, f64, f64, f64) {
    // Design a high-shelf with ~4dB boost above 1500Hz
    // Using the cookbook formula for a high shelf
    let gain_db = 3.999843853973347;
    let a_lin = 10.0_f64.powf(gain_db / 40.0); // sqrt of linear gain
    let freq = 1681.974450955533;
    let q = 0.7071752369554196;

    let w0 = 2.0 * std::f64::consts::PI * freq / sr;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);

    let a0 = (a_lin + 1.0) - (a_lin - 1.0) * cos_w0 + 2.0 * a_lin.sqrt() * alpha;
    let b0 = (a_lin * ((a_lin + 1.0) + (a_lin - 1.0) * cos_w0 + 2.0 * a_lin.sqrt() * alpha)) / a0;
    let b1 = (-2.0 * a_lin * ((a_lin - 1.0) + (a_lin + 1.0) * cos_w0)) / a0;
    let b2 = (a_lin * ((a_lin + 1.0) + (a_lin - 1.0) * cos_w0 - 2.0 * a_lin.sqrt() * alpha)) / a0;
    let a1 = (2.0 * ((a_lin - 1.0) - (a_lin + 1.0) * cos_w0)) / a0;
    let a2 = ((a_lin + 1.0) - (a_lin - 1.0) * cos_w0 - 2.0 * a_lin.sqrt() * alpha) / a0;

    (b0, b1, b2, a1, a2)
}

/// K-weighting stage 2: highpass at ~38 Hz.
fn k_weight_highpass(sr: f64) -> (f64, f64, f64, f64, f64) {
    let freq = 38.13547087602444;
    let q = 0.5003270373238773;

    let w0 = 2.0 * std::f64::consts::PI * freq / sr;
    let cos_w0 = w0.cos();
    let alpha = w0.sin() / (2.0 * q);

    let a0 = 1.0 + alpha;
    let b0 = ((1.0 + cos_w0) / 2.0) / a0;
    let b1 = (-(1.0 + cos_w0)) / a0;
    let b2 = b0;
    let a1 = (-2.0 * cos_w0) / a0;
    let a2 = (1.0 - alpha) / a0;

    (b0, b1, b2, a1, a2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_is_neg_infinity() {
        let silence = vec![0.0f32; 44100];
        let lufs = measure_lufs(&silence, 44100.0);
        assert!(lufs.is_infinite() && lufs < 0.0);
    }

    #[test]
    fn test_sine_loudness_plausible() {
        // 1kHz sine at full scale should be around -3 LUFS
        let sr = 44100.0;
        let samples: Vec<f32> = (0..44100 * 4)
            .map(|i| (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / sr).sin() as f32)
            .collect();
        let lufs = measure_lufs(&samples, sr);
        // Full-scale 1kHz sine should be roughly -3.01 LUFS (with K-weighting boost)
        // Allow a generous range since K-weighting shifts it
        assert!(lufs > -5.0 && lufs < 0.0, "unexpected LUFS: {lufs}");
    }

    #[test]
    fn test_true_peak() {
        let samples = vec![0.0, 0.5, -0.8, 0.3];
        let peak = true_peak_dbfs(&samples);
        let expected = 20.0 * (0.8_f64).log10();
        assert!((peak - expected).abs() < 0.01);
    }
}
