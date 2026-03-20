//! Custom AudioNode implementations for delay and reverb effects.
//!
//! These are built from scratch rather than wrapping library primitives,
//! giving us full control over the algorithms and parameters.

use fundsp::hacker::*;

// ---------------------------------------------------------------------------
// Feedback Delay
// ---------------------------------------------------------------------------

/// Feedback delay line with one-pole lowpass damping in the feedback path.
///
/// - `time`: delay time in seconds
/// - `feedback`: 0.0–1.0, recirculation amount
/// - `mix`: 0.0–1.0, dry/wet blend
///
/// Damping is derived from feedback: higher feedback → more HF rolloff,
/// which prevents harsh ringing and sounds natural.
#[derive(Clone)]
pub struct FeedbackDelay {
    buffer: Vec<f32>,
    write_pos: usize,
    delay_samples: usize,
    feedback: f32,
    damping: f32,
    prev_filtered: f32,
    mix: f32,
    sample_rate: f64,
    delay_seconds: f64,
}

impl FeedbackDelay {
    pub fn new(delay_seconds: f64, feedback: f32, mix: f32) -> Self {
        let feedback = feedback.clamp(0.0, 0.99);
        let mix = mix.clamp(0.0, 1.0);
        let damping = 0.3 + 0.4 * feedback;
        let mut node = FeedbackDelay {
            buffer: Vec::new(),
            write_pos: 0,
            delay_samples: 1,
            feedback,
            damping,
            prev_filtered: 0.0,
            mix,
            sample_rate: 0.0,
            delay_seconds,
        };
        node.set_sample_rate(DEFAULT_SR);
        node
    }
}

impl AudioNode for FeedbackDelay {
    const ID: u64 = 1000;
    type Inputs = U1;
    type Outputs = U1;

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
        self.prev_filtered = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.delay_samples = (self.delay_seconds * sample_rate).round().max(1.0) as usize;
            self.buffer.resize(self.delay_samples + 1, 0.0);
            self.reset();
        }
    }

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let buf_len = self.buffer.len();
        let read_pos = (self.write_pos + buf_len - self.delay_samples) % buf_len;
        let delayed = self.buffer[read_pos];

        // One-pole lowpass in feedback path
        self.prev_filtered = delayed + self.damping * (self.prev_filtered - delayed);

        // Write input + damped feedback into buffer
        self.buffer[self.write_pos] = input[0] + self.feedback * self.prev_filtered;
        self.write_pos = (self.write_pos + 1) % buf_len;

        // Dry/wet mix
        let out = (1.0 - self.mix) * input[0] + self.mix * delayed;
        [out].into()
    }
}

// ---------------------------------------------------------------------------
// Freeverb
// ---------------------------------------------------------------------------

/// Freeverb reference delay lengths at 44100 Hz.
const COMB_LENGTHS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_LENGTHS: [usize; 4] = [556, 441, 341, 225];
const ALLPASS_FEEDBACK: f32 = 0.5;

#[derive(Clone)]
struct CombFilter {
    buffer: Vec<f32>,
    index: usize,
    filtered: f32,
}

impl CombFilter {
    fn new(length: usize) -> Self {
        CombFilter {
            buffer: vec![0.0; length],
            index: 0,
            filtered: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32, room_size: f32, damping: f32) -> f32 {
        let buf_out = self.buffer[self.index];
        self.filtered = buf_out * (1.0 - damping) + self.filtered * damping;
        self.buffer[self.index] = input + room_size * self.filtered;
        self.index += 1;
        if self.index >= self.buffer.len() {
            self.index = 0;
        }
        buf_out
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.index = 0;
        self.filtered = 0.0;
    }
}

#[derive(Clone)]
struct AllpassFilter {
    buffer: Vec<f32>,
    index: usize,
}

impl AllpassFilter {
    fn new(length: usize) -> Self {
        AllpassFilter {
            buffer: vec![0.0; length],
            index: 0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let buf_out = self.buffer[self.index];
        self.buffer[self.index] = input + ALLPASS_FEEDBACK * buf_out;
        self.index += 1;
        if self.index >= self.buffer.len() {
            self.index = 0;
        }
        buf_out - ALLPASS_FEEDBACK * input
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.index = 0;
    }
}

/// Freeverb — classic algorithmic reverb.
///
/// 8 parallel comb filters (with one-pole damping) summed together,
/// then processed through 4 series allpass filters for diffusion.
///
/// - `room_size`: 0.0–1.0, scales comb filter feedback
/// - `damping`: 0.0–1.0, high-frequency absorption
/// - `mix`: 0.0–1.0, dry/wet blend
#[derive(Clone)]
pub struct Freeverb {
    combs: [CombFilter; 8],
    allpasses: [AllpassFilter; 4],
    room_size: f32,
    damping: f32,
    mix: f32,
    sample_rate: f64,
}

impl Freeverb {
    pub fn new(room_size: f32, damping: f32, mix: f32) -> Self {
        let room_size = room_size.clamp(0.0, 1.0);
        let damping = damping.clamp(0.0, 1.0);
        let mix = mix.clamp(0.0, 1.0);
        let mut node = Freeverb {
            combs: COMB_LENGTHS.map(CombFilter::new),
            allpasses: ALLPASS_LENGTHS.map(AllpassFilter::new),
            room_size,
            damping,
            mix,
            sample_rate: 0.0,
        };
        node.set_sample_rate(DEFAULT_SR);
        node
    }

    fn scale_length(base: usize, sample_rate: f64) -> usize {
        ((base as f64) * sample_rate / 44100.0).round().max(1.0) as usize
    }
}

impl AudioNode for Freeverb {
    const ID: u64 = 1001;
    type Inputs = U1;
    type Outputs = U1;

    fn reset(&mut self) {
        for c in &mut self.combs {
            c.reset();
        }
        for a in &mut self.allpasses {
            a.reset();
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            for (i, c) in self.combs.iter_mut().enumerate() {
                let len = Self::scale_length(COMB_LENGTHS[i], sample_rate);
                *c = CombFilter::new(len);
            }
            for (i, a) in self.allpasses.iter_mut().enumerate() {
                let len = Self::scale_length(ALLPASS_LENGTHS[i], sample_rate);
                *a = AllpassFilter::new(len);
            }
        }
    }

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let inp = input[0];

        // Sum 8 parallel comb filters
        let mut comb_sum = 0.0f32;
        for comb in &mut self.combs {
            comb_sum += comb.process(inp, self.room_size, self.damping);
        }
        // Scale down the comb sum to avoid clipping
        comb_sum *= 0.125;

        // Series of 4 allpass filters for diffusion
        let mut signal = comb_sum;
        for ap in &mut self.allpasses {
            signal = ap.process(signal);
        }

        // Dry/wet mix
        let out = (1.0 - self.mix) * inp + self.mix * signal;
        [out].into()
    }
}

// ---------------------------------------------------------------------------
// Compressor
// ---------------------------------------------------------------------------

/// Dynamic range compressor with envelope follower.
///
/// - `threshold`: level in dB above which compression kicks in (e.g., -20.0)
/// - `ratio`: compression ratio (e.g., 4.0 means 4:1)
/// - `attack`: how fast the compressor reacts when signal exceeds threshold (seconds)
/// - `release`: how fast the compressor lets go when signal drops below threshold (seconds)
///
/// Uses a peak-detecting envelope follower with separate attack/release smoothing,
/// then applies gain reduction based on the threshold and ratio.
#[derive(Clone)]
pub struct Compressor {
    threshold: f32,    // dB
    ratio: f32,        // e.g., 4.0 for 4:1
    attack_coeff: f32,
    release_coeff: f32,
    envelope: f32,     // current envelope level (linear)
    sample_rate: f64,
    attack_secs: f64,
    release_secs: f64,
}

impl Compressor {
    pub fn new(threshold_db: f32, ratio: f32, attack_secs: f64, release_secs: f64) -> Self {
        let ratio = ratio.max(1.0);
        let mut comp = Compressor {
            threshold: threshold_db,
            ratio,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            envelope: 0.0,
            sample_rate: 0.0,
            attack_secs,
            release_secs,
        };
        comp.set_sample_rate(DEFAULT_SR);
        comp
    }

    fn compute_coeffs(time_secs: f64, sample_rate: f64) -> f32 {
        if time_secs <= 0.0 || sample_rate <= 0.0 {
            return 0.0;
        }
        (-1.0 / (time_secs * sample_rate)).exp() as f32
    }
}

impl AudioNode for Compressor {
    const ID: u64 = 1002;
    type Inputs = U1;
    type Outputs = U1;

    fn reset(&mut self) {
        self.envelope = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.attack_coeff = Self::compute_coeffs(self.attack_secs, sample_rate);
            self.release_coeff = Self::compute_coeffs(self.release_secs, sample_rate);
        }
    }

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let x = input[0];
        let abs_x = x.abs();

        // Envelope follower: fast attack, slow release
        if abs_x > self.envelope {
            self.envelope = self.attack_coeff * self.envelope + (1.0 - self.attack_coeff) * abs_x;
        } else {
            self.envelope = self.release_coeff * self.envelope + (1.0 - self.release_coeff) * abs_x;
        }

        // Convert envelope to dB
        let env_db = if self.envelope > 1e-10 {
            20.0 * self.envelope.log10()
        } else {
            -200.0 // effectively silence
        };

        // Compute gain reduction
        let gain_db = if env_db > self.threshold {
            let over = env_db - self.threshold;
            let compressed_over = over / self.ratio;
            self.threshold + compressed_over - env_db
        } else {
            0.0
        };

        // Apply gain (convert dB back to linear)
        let gain = (10.0_f32).powf(gain_db / 20.0);
        [x * gain].into()
    }
}

// ---------------------------------------------------------------------------
// Wavetable Oscillator
// ---------------------------------------------------------------------------

/// Wavetable oscillator — reads through a user-defined array of sample points
/// at the right speed for the desired frequency, interpolating linearly.
///
/// - `table`: one cycle of the waveform as sample values (typically -1.0 to 1.0)
/// - `freq`: playback frequency in Hz
///
/// 0 inputs, 1 output (source oscillator).
#[derive(Clone)]
pub struct WavetableOsc {
    table: Vec<f32>,
    phase: f64,     // 0.0 to 1.0
    freq: f32,
    sample_rate: f64,
}

impl WavetableOsc {
    pub fn new(samples: &[f64], freq: f32) -> Self {
        let table: Vec<f32> = samples.iter().map(|s| *s as f32).collect();
        WavetableOsc {
            table,
            phase: 0.0,
            freq,
            sample_rate: DEFAULT_SR,
        }
    }
}

impl AudioNode for WavetableOsc {
    const ID: u64 = 1003;
    type Inputs = U0;
    type Outputs = U1;

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
    }

    #[inline]
    fn tick(&mut self, _input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let len = self.table.len();
        if len == 0 {
            return [0.0].into();
        }

        // Fractional index into the table
        let pos = self.phase * len as f64;
        let idx = pos as usize;
        let frac = pos - idx as f64;

        // Linear interpolation between adjacent samples (wrapping)
        let a = self.table[idx % len];
        let b = self.table[(idx + 1) % len];
        let sample = a + (b - a) * frac as f32;

        // Advance phase
        self.phase += self.freq as f64 / self.sample_rate;
        // Wrap phase to avoid floating point drift over time
        self.phase -= self.phase.floor();

        [sample].into()
    }
}

// ---------------------------------------------------------------------------
// Leaky Filter (lowpass with dry/wet mix)
// ---------------------------------------------------------------------------

/// One-pole lowpass filter with dry/wet mix.
/// At mix=1.0, fully filtered. At mix=0.5, half the original leaks through.
///
/// This is simpler than the SVF lowpass_hz but allows partial filtering
/// without the routing complexity of parallel dry/wet paths.
///
/// 1 input, 1 output.
#[derive(Clone)]
pub struct LeakyFilter {
    coeff: f32,
    state: f32,
    mix: f32,
    sample_rate: f64,
    cutoff: f32,
}

impl LeakyFilter {
    pub fn new(cutoff: f32, mix: f32) -> Self {
        let coeff = Self::calc_coeff(cutoff, DEFAULT_SR);
        LeakyFilter {
            coeff,
            state: 0.0,
            mix: mix.max(0.0).min(1.0),
            sample_rate: DEFAULT_SR,
            cutoff,
        }
    }

    fn calc_coeff(cutoff: f32, sr: f64) -> f32 {
        (-2.0 * std::f32::consts::PI * cutoff / sr as f32).exp()
    }
}

impl AudioNode for LeakyFilter {
    const ID: u64 = 1007;
    type Inputs = U1;
    type Outputs = U1;

    fn reset(&mut self) {
        self.state = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.coeff = Self::calc_coeff(self.cutoff, sample_rate);
        }
    }

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let dry = input[0];
        // One-pole lowpass
        self.state = self.coeff * self.state + (1.0 - self.coeff) * dry;
        // Blend: mix=1.0 fully filtered, mix=0.0 fully dry
        let out = dry * (1.0 - self.mix) + self.state * self.mix;
        [out].into()
    }
}

// ---------------------------------------------------------------------------
// Bit Crusher
// ---------------------------------------------------------------------------

/// Reduces bit depth of the signal, creating quantization noise.
///
/// - `bits`: effective bit depth (1.0 = extreme, 8.0 = retro, 16.0 = CD quality)
///
/// Lower values = more destruction. Fractional values are allowed for smooth control.
/// 1 input, 1 output.
#[derive(Clone)]
pub struct BitCrush {
    levels: f32,
}

impl BitCrush {
    pub fn new(bits: f32) -> Self {
        let levels = (2.0_f32).powf(bits.max(1.0).min(16.0));
        BitCrush { levels }
    }
}

impl AudioNode for BitCrush {
    const ID: u64 = 1004;
    type Inputs = U1;
    type Outputs = U1;

    fn reset(&mut self) {}

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let x = input[0];
        // Quantize: map to discrete levels then back
        let crushed = (x * self.levels).round() / self.levels;
        [crushed].into()
    }
}

// ---------------------------------------------------------------------------
// Sample Rate Reducer (Decimator)
// ---------------------------------------------------------------------------

/// Reduces effective sample rate by holding samples, creating aliasing artifacts.
///
/// - `factor`: hold each sample for this many ticks. 1.0 = no effect, 4.0 = quarter rate,
///   10.0+ = heavy digital degradation.
///
/// 1 input, 1 output.
#[derive(Clone)]
pub struct Decimate {
    factor: f32,
    counter: f32,
    held: f32,
}

impl Decimate {
    pub fn new(factor: f32) -> Self {
        Decimate {
            factor: factor.max(1.0),
            counter: 0.0,
            held: 0.0,
        }
    }
}

impl AudioNode for Decimate {
    const ID: u64 = 1005;
    type Inputs = U1;
    type Outputs = U1;

    fn reset(&mut self) {
        self.counter = 0.0;
        self.held = 0.0;
    }

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        self.counter += 1.0;
        if self.counter >= self.factor {
            self.counter -= self.factor;
            self.held = input[0];
        }
        [self.held].into()
    }
}

// ---------------------------------------------------------------------------
// Degrade — combined tape/medium degradation effect
// ---------------------------------------------------------------------------

/// Simulates signal degradation through a worn medium (tape, phone line, radio).
///
/// Combines lowpass filtering, sample rate reduction, bit crushing, and noise
/// replacement in a single effect. The `amount` parameter (0.0–1.0) controls
/// the intensity of all degradation stages together.
///
/// - amount 0.0 = clean signal
/// - amount 0.3 = subtle warmth, slight noise floor
/// - amount 0.6 = worn tape, noticeable degradation
/// - amount 1.0 = destroyed, mostly noise
///
/// Internally:
/// - Lowpass cutoff: 8000 Hz → 400 Hz as amount increases
/// - Decimate factor: 1 → 8 as amount increases
/// - Bit crush: 14 → 4 bits as amount increases
/// - Noise mix: 0 → 15% as amount increases
///
/// 1 input, 1 output.
#[derive(Clone)]
pub struct Degrade {
    // Lowpass state (one-pole)
    lp_coeff: f32,
    lp_state: f32,
    // Decimation
    decimate_factor: f32,
    dec_counter: f32,
    dec_held: f32,
    // Bit crush
    crush_levels: f32,
    // Noise mix
    noise_amount: f32,
    noise_state: u32, // simple PRNG state
    // Stored for set_sample_rate recalculation
    amount: f32,
    sample_rate: f64,
}

impl Degrade {
    pub fn new(amount: f32) -> Self {
        let amount = amount.max(0.0).min(1.0);
        let mut d = Degrade {
            lp_coeff: 0.0,
            lp_state: 0.0,
            decimate_factor: 1.0 + amount * 7.0,
            dec_counter: 0.0,
            dec_held: 0.0,
            crush_levels: (2.0_f32).powf(14.0 - amount * 10.0),
            noise_amount: amount * 0.15,
            noise_state: 12345,
            amount,
            sample_rate: DEFAULT_SR,
        };
        d.recalc_coeff();
        d
    }

    fn recalc_coeff(&mut self) {
        let cutoff = 8000.0 * (1.0 - self.amount) + 400.0 * self.amount;
        self.lp_coeff = (-2.0 * std::f32::consts::PI * cutoff / self.sample_rate as f32).exp();
    }

    #[inline]
    fn next_noise(&mut self) -> f32 {
        // Simple xorshift PRNG for deterministic noise
        self.noise_state ^= self.noise_state << 13;
        self.noise_state ^= self.noise_state >> 17;
        self.noise_state ^= self.noise_state << 5;
        // Map to -1.0..1.0
        (self.noise_state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

impl AudioNode for Degrade {
    const ID: u64 = 1006;
    type Inputs = U1;
    type Outputs = U1;

    fn reset(&mut self) {
        self.lp_state = 0.0;
        self.dec_counter = 0.0;
        self.dec_held = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.recalc_coeff();
        }
    }

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let x = input[0];

        // 1. One-pole lowpass
        self.lp_state = self.lp_coeff * self.lp_state + (1.0 - self.lp_coeff) * x;
        let filtered = self.lp_state;

        // 2. Decimate (sample-and-hold)
        self.dec_counter += 1.0;
        if self.dec_counter >= self.decimate_factor {
            self.dec_counter -= self.decimate_factor;
            self.dec_held = filtered;
        }
        let decimated = self.dec_held;

        // 3. Bit crush
        let crushed = (decimated * self.crush_levels).round() / self.crush_levels;

        // 4. Mix in noise (replacing lost signal content)
        let noise = self.next_noise();
        let out = crushed * (1.0 - self.noise_amount) + noise * self.noise_amount;

        [out].into()
    }
}

// ---------------------------------------------------------------------------
// Noise Gate
// ---------------------------------------------------------------------------

/// Silences signal below a threshold, with attack/release smoothing.
///
/// - `threshold`: level below which signal is muted (0.0-1.0 linear, e.g. 0.01)
/// - `attack`: how fast the gate opens when signal exceeds threshold (seconds)
/// - `release`: how fast the gate closes when signal drops below (seconds)
///
/// 1 input, 1 output.
#[derive(Clone)]
pub struct NoiseGate {
    threshold: f32,
    attack_coeff: f32,
    release_coeff: f32,
    envelope: f32,
    sample_rate: f64,
    attack_secs: f64,
    release_secs: f64,
}

impl NoiseGate {
    pub fn new(threshold: f32, attack: f32, release: f32) -> Self {
        let mut g = NoiseGate {
            threshold,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            envelope: 0.0,
            sample_rate: DEFAULT_SR,
            attack_secs: attack as f64,
            release_secs: release as f64,
        };
        g.recalc();
        g
    }

    fn recalc(&mut self) {
        self.attack_coeff = (-1.0 / (self.attack_secs * self.sample_rate)).exp() as f32;
        self.release_coeff = (-1.0 / (self.release_secs * self.sample_rate)).exp() as f32;
    }
}

impl AudioNode for NoiseGate {
    const ID: u64 = 1008;
    type Inputs = U1;
    type Outputs = U1;

    fn reset(&mut self) {
        self.envelope = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.recalc();
        }
    }

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let x = input[0];
        let abs_x = x.abs();

        // Envelope follower
        if abs_x > self.envelope {
            self.envelope = self.attack_coeff * self.envelope + (1.0 - self.attack_coeff) * abs_x;
        } else {
            self.envelope = self.release_coeff * self.envelope + (1.0 - self.release_coeff) * abs_x;
        }

        // Gate: pass signal if envelope is above threshold, silence otherwise
        let gate = if self.envelope > self.threshold { 1.0 } else { 0.0 };
        [x * gate].into()
    }
}
