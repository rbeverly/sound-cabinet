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
/// Stereo feedback delay with slightly offset L/R times for width.
/// Inputs: 2 (stereo), Outputs: 2 (stereo).
/// When fed mono (via auto-duplicate), the offset delay times create stereo spread.
pub struct FeedbackDelay {
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    write_pos_l: usize,
    write_pos_r: usize,
    delay_samples_l: usize,
    delay_samples_r: usize,
    feedback: f32,
    damping: f32,
    prev_filtered_l: f32,
    prev_filtered_r: f32,
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
            buf_l: Vec::new(),
            buf_r: Vec::new(),
            write_pos_l: 0,
            write_pos_r: 0,
            delay_samples_l: 1,
            delay_samples_r: 1,
            feedback,
            damping,
            prev_filtered_l: 0.0,
            prev_filtered_r: 0.0,
            mix,
            sample_rate: 0.0,
            delay_seconds,
        };
        node.set_sample_rate(DEFAULT_SR);
        node
    }

    #[inline]
    fn process_channel(
        buf: &mut [f32],
        write_pos: &mut usize,
        delay_samples: usize,
        prev_filtered: &mut f32,
        feedback: f32,
        damping: f32,
        input: f32,
        mix: f32,
    ) -> f32 {
        let buf_len = buf.len();
        let read_pos = (*write_pos + buf_len - delay_samples) % buf_len;
        let delayed = buf[read_pos];
        *prev_filtered = delayed + damping * (*prev_filtered - delayed);
        buf[*write_pos] = input + feedback * *prev_filtered;
        *write_pos = (*write_pos + 1) % buf_len;
        (1.0 - mix) * input + mix * delayed
    }
}

impl AudioNode for FeedbackDelay {
    const ID: u64 = 1000;
    type Inputs = U2;
    type Outputs = U2;

    fn reset(&mut self) {
        self.buf_l.fill(0.0);
        self.buf_r.fill(0.0);
        self.write_pos_l = 0;
        self.write_pos_r = 0;
        self.prev_filtered_l = 0.0;
        self.prev_filtered_r = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.delay_samples_l = (self.delay_seconds * sample_rate).round().max(1.0) as usize;
            // Right channel: ~7% longer delay for stereo width
            self.delay_samples_r = ((self.delay_seconds * 1.07) * sample_rate).round().max(1.0) as usize;
            self.buf_l.resize(self.delay_samples_l + 1, 0.0);
            self.buf_r.resize(self.delay_samples_r + 1, 0.0);
            self.reset();
        }
    }

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let out_l = Self::process_channel(
            &mut self.buf_l, &mut self.write_pos_l, self.delay_samples_l,
            &mut self.prev_filtered_l, self.feedback, self.damping, input[0], self.mix,
        );
        let out_r = Self::process_channel(
            &mut self.buf_r, &mut self.write_pos_r, self.delay_samples_r,
            &mut self.prev_filtered_r, self.feedback, self.damping, input[1], self.mix,
        );
        [out_l, out_r].into()
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

/// Stereo spread offset for right channel comb filters (in samples at 44100 Hz).
/// This decorrelates L/R reflections, creating natural stereo width.
const STEREO_SPREAD: usize = 23;

/// Freeverb — stereo algorithmic reverb.
///
/// Two independent banks of 8 parallel comb filters + 4 series allpass filters,
/// one for each channel. The right channel uses slightly longer delay lines
/// (offset by STEREO_SPREAD samples) to decorrelate the reflections, creating
/// natural stereo width even from a mono input.
///
/// Inputs: 2 (stereo), Outputs: 2 (stereo).
///
/// - `room_size`: 0.0–1.0, scales comb filter feedback
/// - `damping`: 0.0–1.0, high-frequency absorption
/// - `mix`: 0.0–1.0, dry/wet blend
#[derive(Clone)]
pub struct Freeverb {
    combs_l: [CombFilter; 8],
    combs_r: [CombFilter; 8],
    allpasses_l: [AllpassFilter; 4],
    allpasses_r: [AllpassFilter; 4],
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
            combs_l: COMB_LENGTHS.map(CombFilter::new),
            combs_r: COMB_LENGTHS.map(|len| CombFilter::new(len + STEREO_SPREAD)),
            allpasses_l: ALLPASS_LENGTHS.map(AllpassFilter::new),
            allpasses_r: ALLPASS_LENGTHS.map(|len| AllpassFilter::new(len + STEREO_SPREAD)),
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

    #[inline]
    fn process_channel(
        combs: &mut [CombFilter; 8],
        allpasses: &mut [AllpassFilter; 4],
        input: f32,
        room_size: f32,
        damping: f32,
        mix: f32,
    ) -> f32 {
        let mut comb_sum = 0.0f32;
        for comb in combs.iter_mut() {
            comb_sum += comb.process(input, room_size, damping);
        }
        comb_sum *= 0.125;

        let mut signal = comb_sum;
        for ap in allpasses.iter_mut() {
            signal = ap.process(signal);
        }

        (1.0 - mix) * input + mix * signal
    }
}

impl AudioNode for Freeverb {
    const ID: u64 = 1001;
    type Inputs = U2;
    type Outputs = U2;

    fn reset(&mut self) {
        for c in self.combs_l.iter_mut().chain(self.combs_r.iter_mut()) {
            c.reset();
        }
        for a in self.allpasses_l.iter_mut().chain(self.allpasses_r.iter_mut()) {
            a.reset();
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            for (i, c) in self.combs_l.iter_mut().enumerate() {
                let len = Self::scale_length(COMB_LENGTHS[i], sample_rate);
                *c = CombFilter::new(len);
            }
            for (i, c) in self.combs_r.iter_mut().enumerate() {
                let len = Self::scale_length(COMB_LENGTHS[i] + STEREO_SPREAD, sample_rate);
                *c = CombFilter::new(len);
            }
            for (i, a) in self.allpasses_l.iter_mut().enumerate() {
                let len = Self::scale_length(ALLPASS_LENGTHS[i], sample_rate);
                *a = AllpassFilter::new(len);
            }
            for (i, a) in self.allpasses_r.iter_mut().enumerate() {
                let len = Self::scale_length(ALLPASS_LENGTHS[i] + STEREO_SPREAD, sample_rate);
                *a = AllpassFilter::new(len);
            }
        }
    }

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let out_l = Self::process_channel(
            &mut self.combs_l, &mut self.allpasses_l,
            input[0], self.room_size, self.damping, self.mix,
        );
        let out_r = Self::process_channel(
            &mut self.combs_r, &mut self.allpasses_r,
            input[1], self.room_size, self.damping, self.mix,
        );
        [out_l, out_r].into()
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
// Expander (opposite of compressor — increases dynamic range)
// ---------------------------------------------------------------------------

/// Downward expander — reduces signal below threshold, making quiet parts quieter.
/// Increases dynamic range and perceived clarity.
///
/// - `threshold`: level in dB below which expansion kicks in (e.g., -30.0)
/// - `ratio`: expansion ratio (e.g., 2.0 means 1:2 — for every 1 dB below threshold, output drops 2 dB)
/// - `attack`: how fast the expander reacts (seconds)
/// - `release`: how fast it recovers (seconds)
#[derive(Clone)]
pub struct Expander {
    threshold: f32,
    ratio: f32,
    attack_coeff: f32,
    release_coeff: f32,
    envelope: f32,
    sample_rate: f64,
    attack_secs: f64,
    release_secs: f64,
    knee_width: f32,   // soft knee width in dB
}

impl Expander {
    pub fn new(threshold_db: f32, ratio: f32, attack_secs: f64, release_secs: f64) -> Self {
        let mut exp = Expander {
            threshold: threshold_db,
            ratio: ratio.max(1.0),
            attack_coeff: 0.0,
            release_coeff: 0.0,
            envelope: 0.0,
            sample_rate: 0.0,
            attack_secs,
            release_secs,
            knee_width: 6.0,
        };
        exp.set_sample_rate(DEFAULT_SR);
        exp
    }
}

impl AudioNode for Expander {
    const ID: u64 = 1013;
    type Inputs = U1;
    type Outputs = U1;

    fn reset(&mut self) {
        self.envelope = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.attack_coeff = (-1.0 / (self.attack_secs * sample_rate)).exp() as f32;
        self.release_coeff = (-1.0 / (self.release_secs * sample_rate)).exp() as f32;
    }

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let x = input[0];
        let abs = x.abs();

        // Peak envelope follower
        if abs > self.envelope {
            self.envelope = self.attack_coeff * self.envelope + (1.0 - self.attack_coeff) * abs;
        } else {
            self.envelope = self.release_coeff * self.envelope + (1.0 - self.release_coeff) * abs;
        }

        if self.envelope < 1e-10 { return [0.0].into(); }

        let env_db = 20.0 * self.envelope.log10();

        // Expansion with soft knee: reduce signal BELOW threshold
        let knee = self.knee_width;
        let gain_db = if env_db > self.threshold + knee / 2.0 {
            // Above knee: no expansion
            0.0
        } else if env_db < self.threshold - knee / 2.0 {
            // Below knee: full expansion
            let under = self.threshold - env_db;
            -(under * (self.ratio - 1.0)) // expand: multiply the undershoot
        } else {
            // In knee zone: smooth transition from full expansion to no expansion
            let x = self.threshold + knee / 2.0 - env_db;
            -(self.ratio - 1.0) * x * x / (2.0 * knee)
        };

        let gain = 10.0_f32.powf(gain_db.max(-60.0) / 20.0); // cap at -60 dB reduction
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
        // Use exponential curves so low amounts are subtle and high amounts are destructive.
        // amount 0.0-0.3: barely noticeable warmth
        // amount 0.3-0.6: audible degradation
        // amount 0.6-1.0: heavy destruction
        let amt_sq = amount * amount; // quadratic curve — gentler at low values
        let mut d = Degrade {
            lp_coeff: 0.0,
            lp_state: 0.0,
            decimate_factor: 1.0 + amt_sq * 7.0,        // 1.0 at 0, ~1.07 at 0.3, ~2.5 at 0.6, 8.0 at 1.0
            dec_counter: 0.0,
            dec_held: 0.0,
            crush_levels: (2.0_f32).powf(14.0 - amt_sq * 10.0), // 14-bit at 0, ~13-bit at 0.3, ~10-bit at 0.6, 4-bit at 1.0
            noise_amount: amt_sq * amt_sq * 0.15,        // essentially zero below 0.3, ~0.001 at 0.5, 0.15 at 1.0
            noise_state: 12345,
            amount,
            sample_rate: DEFAULT_SR,
        };
        d.recalc_coeff();
        d
    }

    fn recalc_coeff(&mut self) {
        let amt_sq = self.amount * self.amount;
        let cutoff = 8000.0 * (1.0 - amt_sq) + 400.0 * amt_sq;
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
// Brick-Wall Limiter
// ---------------------------------------------------------------------------

/// Brick-wall peak limiter with lookahead and release smoothing.
///
/// Prevents signal from exceeding `ceiling` (linear). Uses a short lookahead
// ---------------------------------------------------------------------------
// Harmonic Exciter
// ---------------------------------------------------------------------------

/// Harmonic exciter — extracts high-frequency content via highpass, applies
/// tanh saturation to generate new harmonics, then blends the excited signal
/// back with the dry input.
///
/// Unlike a simple high shelf boost, this *creates* harmonics that weren't
/// in the original signal. A 2 kHz tone through the exciter generates
/// 4 kHz, 6 kHz, 8 kHz partials that add "air" and presence.
///
/// - `cutoff_hz`: frequency above which to excite (e.g., 3000)
/// - `drive`: saturation amount (2.0 = gentle, 8.0 = aggressive)
/// - `blend`: how much excited signal to mix in (0.0-1.0)
#[derive(Clone)]
pub struct HarmonicExciter {
    // Highpass filter state (one-pole for simplicity)
    hp_coeff: f32,
    hp_state: f32,
    drive: f32,
    drive_norm: f32, // 1.0 / tanh(drive) for unity gain normalization
    blend: f32,
    sample_rate: f64,
    cutoff_hz: f32,
}

impl HarmonicExciter {
    pub fn new(cutoff_hz: f32, amount: f32) -> Self {
        let drive = 2.0 + amount.clamp(0.0, 1.0) * 6.0; // 2.0 to 8.0
        let blend = amount.clamp(0.0, 1.0) * 0.4; // 0.0 to 0.4 (subtle blend)
        let mut exc = HarmonicExciter {
            hp_coeff: 0.0,
            hp_state: 0.0,
            drive,
            drive_norm: 1.0 / drive.tanh(),
            blend,
            sample_rate: DEFAULT_SR,
            cutoff_hz,
        };
        exc.recalc_coeff();
        exc
    }

    fn recalc_coeff(&mut self) {
        // One-pole highpass coefficient
        let rc = 1.0 / (2.0 * std::f64::consts::PI * self.cutoff_hz as f64);
        let dt = 1.0 / self.sample_rate;
        self.hp_coeff = (rc / (rc + dt)) as f32;
    }
}

impl AudioNode for HarmonicExciter {
    const ID: u64 = 1012;
    type Inputs = U1;
    type Outputs = U1;

    fn reset(&mut self) {
        self.hp_state = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.recalc_coeff();
    }

    #[inline]
    fn tick(&mut self, input: &Frame<f32, Self::Inputs>) -> Frame<f32, Self::Outputs> {
        let x = input[0];

        // One-pole highpass: extract high-frequency content
        let hp = self.hp_coeff * (self.hp_state + x);
        self.hp_state = hp - x;
        let highs = hp;

        // Saturate the highs to generate harmonics
        let saturated = (highs * self.drive).tanh() * self.drive_norm;

        // Blend excited signal with dry input
        let out = x + saturated * self.blend;
        [out].into()
    }
}

// ---------------------------------------------------------------------------
// Brick-wall Limiter
// ---------------------------------------------------------------------------

/// to catch transients cleanly without harsh clipping artifacts.
///
/// - `ceiling`: maximum output level (linear, e.g. 0.97 for -0.3 dBFS)
/// - `release`: how fast gain recovers after limiting (seconds)
///
/// Not an AudioNode — operates on buffers directly for use on the master bus.
#[derive(Clone)]
pub struct BrickwallLimiter {
    pub ceiling: f32,
    pub release_coeff: f32,
    pub gain_reduction: f32, // current gain reduction (0.0 = no reduction, higher = more)
    lookahead_buf_l: Vec<f32>,
    lookahead_buf_r: Vec<f32>,
    lookahead_pos: usize,
    lookahead_len: usize,
    peak_hold_timer: usize,
    history_l: [f32; 3], // for 4-point hermite true peak interpolation
    history_r: [f32; 3],
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
            history_l: [0.0; 3],
            history_r: [0.0; 3],
        }
    }

    #[inline(always)]
    fn hermite(y0: f32, y1: f32, y2: f32, y3: f32, mu: f32) -> f32 {
        let mu2 = mu * mu;
        let mu3 = mu2 * mu;
        let m0 = (y1 - y0) * 0.5 + (y2 - y1) * 0.5;
        let m1 = (y2 - y1) * 0.5 + (y3 - y2) * 0.5;
        let a0 = 2.0 * mu3 - 3.0 * mu2 + 1.0;
        let a1 = mu3 - 2.0 * mu2 + mu;
        let a2 = mu3 - mu2;
        let a3 = -2.0 * mu3 + 3.0 * mu2;
        a0 * y1 + a1 * m0 + a2 * m1 + a3 * y2
    }

    /// Process a stereo buffer of samples in-place.
    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let delayed_l = self.lookahead_buf_l[self.lookahead_pos];
            let delayed_r = self.lookahead_buf_r[self.lookahead_pos];
            self.lookahead_buf_l[self.lookahead_pos] = *l;
            self.lookahead_buf_r[self.lookahead_pos] = *r;
            self.lookahead_pos = (self.lookahead_pos + 1) % self.lookahead_len;

            // Compute required gain reduction using True Peak (4x Hermite interpolation)
            let mut max_abs = l.abs().max(r.abs());
            for mu in [0.25, 0.5, 0.75].iter() {
                let interp_l = Self::hermite(self.history_l[0], self.history_l[1], self.history_l[2], *l, *mu);
                let interp_r = Self::hermite(self.history_r[0], self.history_r[1], self.history_r[2], *r, *mu);
                max_abs = max_abs.max(interp_l.abs()).max(interp_r.abs());
            }
            self.history_l = [self.history_l[1], self.history_l[2], *l];
            self.history_r = [self.history_r[1], self.history_r[2], *r];

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
}

// ---------------------------------------------------------------------------
// Master Compressor (buffer-based, for master bus)
// ---------------------------------------------------------------------------

/// Gentle mastering compressor that reduces crest factor (the gap between
/// peak transients and sustained content). Sits before the limiter to raise
/// the perceived loudness floor without clipping.
///
/// Uses RMS envelope detection (not peak) for musical, transparent compression.
/// Default settings: -18 dB threshold, 2:1 ratio, 10ms attack, 200ms release.
///
/// Not an AudioNode — operates on buffers for master bus use.
#[derive(Clone)]
pub struct MasterCompressor {
    threshold: f32,    // dB
    ratio: f32,
    attack_coeff: f32,
    release_coeff: f32,
    envelope_sq: f32,  // squared RMS envelope (avoids sqrt per sample)
    makeup_gain: f32,  // compensate for gain reduction
    upward: bool,      // upward compression: boost quiet content instead of reducing loud
    knee_width: f32,   // soft knee width in dB (Giannoulis/Massberg/Reiss JAES 2012)
    last_gr_db: f32,   // instantaneous gain reduction for metering
}

impl MasterCompressor {
    pub fn new(threshold_db: f32, ratio: f32, attack_secs: f64, release_secs: f64, sample_rate: f64) -> Self {
        let attack_coeff = (-1.0 / (attack_secs * sample_rate)).exp() as f32;
        let release_coeff = (-1.0 / (release_secs * sample_rate)).exp() as f32;
        // Auto-makeup gain: compensate for average gain reduction.
        // Standard formula: half of the maximum possible reduction at threshold.
        // For -18dB threshold, 2:1 ratio: 18 * (1 - 1/2) / 2 = 4.5 dB makeup
        let makeup_db = threshold_db.abs() * (1.0 - 1.0 / ratio) / 2.0;
        let makeup_gain = 10.0_f32.powf(makeup_db.min(12.0) / 20.0);
        MasterCompressor {
            threshold: threshold_db,
            ratio: ratio.max(1.0),
            attack_coeff,
            release_coeff,
            envelope_sq: 0.0,
            makeup_gain,
            upward: false,
            knee_width: 6.0,
            last_gr_db: 0.0,
        }
    }

    /// Create an upward compressor (boosts quiet content).
    pub fn new_upward(threshold_db: f32, ratio: f32, attack_secs: f64, release_secs: f64, sample_rate: f64) -> Self {
        let mut comp = Self::new(threshold_db, ratio, attack_secs, release_secs, sample_rate);
        comp.upward = true;
        // Upward compression doesn't need makeup gain — it boosts directly
        comp.makeup_gain = 1.0;
        comp
    }

    /// Create from a simple 0.0–2.0 amount parameter.
    ///
    /// - 0.0 = off (1:1 ratio, effectively bypass)
    /// - 0.5 = gentle (threshold -24 dB, 1.5:1)
    /// - 1.0 = default (threshold -18 dB, 2:1)
    /// - 2.0 = heavy (threshold -12 dB, 3:1)
    pub fn from_amount(amount: f32, sample_rate: f64) -> Self {
        if amount <= 0.0 {
            // Bypass: ratio 1:1 means no gain change
            return Self::new(-100.0, 1.0, 0.01, 0.2, sample_rate);
        }
        let amount = amount.min(3.0);
        // Threshold: -24 at 0.5, -18 at 1.0, -12 at 2.0
        let threshold = -18.0 / amount;
        // Ratio: 1.5:1 at 0.5, 2:1 at 1.0, 3:1 at 2.0
        let ratio = 1.0 + amount;
        // Attack: faster at higher amounts (10ms default, 5ms at 2.0)
        let attack = (0.010 / amount.max(0.5)) as f64;
        // Release: 200ms default
        let release = 0.200_f64;
        Self::new(threshold, ratio, attack, release, sample_rate)
    }

    /// Process a buffer in-place.
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
self.last_gr_db = gain_db.min(0.0);
*sample *= gain;
}
}}

// ---------------------------------------------------------------------------
// Master Bus (highpass + lowpass + compressor + limiter)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
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

// ---------------------------------------------------------------------------
// Master Bus Support Types
// ---------------------------------------------------------------------------

/// Per-channel biquad filter state (delay elements only).
#[derive(Clone)]
struct BiquadState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Default for BiquadState {
    fn default() -> Self {
        BiquadState { x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }
}

/// Master bus processing chain: bandpass filter (HP + LP), gentle compressor,
/// and brick-wall limiter. Operates on buffers, not the AudioNode trait.
///
/// - Highpass at 30 Hz: removes inaudible sub-bass that eats headroom
/// - Lowpass at 18 kHz: removes ultrasonic content from aliasing/resonance
/// - Compressor: reduces crest factor (peak-to-RMS gap) for higher perceived loudness
/// - Limiter at -0.3 dBFS: prevents peaks from hitting 0 dBFS
///
/// Supports stereo processing: each channel has independent filter state,
/// compressor, and limiter, but shares filter coefficients.
/// A single biquad EQ band with shared coefficients and per-channel state.
#[derive(Clone)]
struct EqBand {
    b0: f32, b1: f32, b2: f32, a1: f32, a2: f32,
    state: [BiquadState; 2],
    active: bool,
}

impl EqBand {
    fn bypass() -> Self {
        EqBand {
            b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0,
            state: [BiquadState::default(), BiquadState::default()],
            active: false,
        }
    }

    fn low_shelf(freq: f64, gain_db: f32, sr: f64) -> Self {
        if gain_db.abs() < 0.01 { return Self::bypass(); }
        let a = 10.0_f64.powf(gain_db as f64 / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * freq / sr;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * std::f64::consts::SQRT_2);
        let ap1 = a + 1.0;
        let am1 = a - 1.0;
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
        let a0 = ap1 + am1 * cos_w0 + two_sqrt_a_alpha;
        EqBand {
            b0: (a * (ap1 - am1 * cos_w0 + two_sqrt_a_alpha) / a0) as f32,
            b1: (2.0 * a * (am1 - ap1 * cos_w0) / a0) as f32,
            b2: (a * (ap1 - am1 * cos_w0 - two_sqrt_a_alpha) / a0) as f32,
            a1: (-2.0 * (am1 + ap1 * cos_w0) / a0) as f32,
            a2: ((ap1 + am1 * cos_w0 - two_sqrt_a_alpha) / a0) as f32,
            state: [BiquadState::default(), BiquadState::default()],
            active: true,
        }
    }

    fn high_shelf(freq: f64, gain_db: f32, sr: f64) -> Self {
        if gain_db.abs() < 0.01 { return Self::bypass(); }
        let a = 10.0_f64.powf(gain_db as f64 / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * freq / sr;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * std::f64::consts::SQRT_2);
        let ap1 = a + 1.0;
        let am1 = a - 1.0;
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
        let a0 = ap1 - am1 * cos_w0 + two_sqrt_a_alpha;
        EqBand {
            b0: (a * (ap1 + am1 * cos_w0 + two_sqrt_a_alpha) / a0) as f32,
            b1: (-2.0 * a * (am1 + ap1 * cos_w0) / a0) as f32,
            b2: (a * (ap1 + am1 * cos_w0 - two_sqrt_a_alpha) / a0) as f32,
            a1: (2.0 * (am1 - ap1 * cos_w0) / a0) as f32,
            a2: ((ap1 - am1 * cos_w0 - two_sqrt_a_alpha) / a0) as f32,
            state: [BiquadState::default(), BiquadState::default()],
            active: true,
        }
    }

    fn peak(freq: f64, gain_db: f32, q: f64, sr: f64) -> Self {
        if gain_db.abs() < 0.01 { return Self::bypass(); }
        let a = 10.0_f64.powf(gain_db as f64 / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * freq / sr;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alpha / a;
        EqBand {
            b0: ((1.0 + alpha * a) / a0) as f32,
            b1: ((-2.0 * cos_w0) / a0) as f32,
            b2: ((1.0 - alpha * a) / a0) as f32,
            a1: ((-2.0 * cos_w0) / a0) as f32,
            a2: ((1.0 - alpha / a) / a0) as f32,
            state: [BiquadState::default(), BiquadState::default()],
            active: true,
        }
    }

    #[inline]
    fn process_sample(&mut self, x: f32, ch: usize) -> f32 {
        if !self.active { return x; }
        let s = &mut self.state[ch];
        let y = self.b0 * x + self.b1 * s.x1 + self.b2 * s.x2 - self.a1 * s.y1 - self.a2 * s.y2;
        s.x2 = s.x1;
        s.x1 = x;
        s.y2 = s.y1;
        s.y1 = y;
        y
    }
}

// ---------------------------------------------------------------------------
// MasterStage trait and wrapper structs for user-definable master chain
// ---------------------------------------------------------------------------

/// A processing stage in the master bus chain. Each stage operates on
/// stereo buffers in-place. Stages are ordered by the user and executed
/// sequentially between the HP/LP filters and the brick-wall limiter.
pub trait MasterStage: Send {
    fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]);
    fn name(&self) -> &'static str;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    /// Return instantaneous gain reduction in dB
    fn gain_reduction_db(&self) -> f32 { 0.0 }
}

/// Compressor stage — wraps a stereo pair of MasterCompressors.
pub struct StageCompress {
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
}

/// Saturator stage — tanh soft clipper with drive.
pub struct StageSaturate {
    pub drive: f32,
}

impl MasterStage for StageSaturate {
    fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        if self.drive <= 0.0 { return; }
        let drive = self.drive;
        let norm = 1.0 / drive.tanh();
        for sample in left.iter_mut() {
            *sample = (*sample * drive).tanh() * norm;
        }
        for sample in right.iter_mut() {
            *sample = (*sample * drive).tanh() * norm;
        }
    }
    fn name(&self) -> &'static str { "saturate" }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// EQ stage — 3-band master EQ (low shelf, mid peak, high shelf).
pub struct StageEq {
    pub eq_low: EqBand,
    pub eq_mid: EqBand,
    pub eq_high: EqBand,
}

impl StageEq {
    pub fn new(low_db: f32, mid_db: f32, high_db: f32, sample_rate: f64) -> Self {
        StageEq {
            eq_low: EqBand::low_shelf(120.0, low_db, sample_rate),
            eq_mid: EqBand::peak(1000.0, mid_db, 0.7, sample_rate),
            eq_high: EqBand::high_shelf(6000.0, high_db, sample_rate),
        }
    }

    pub fn bypass() -> Self {
        StageEq {
            eq_low: EqBand::bypass(),
            eq_mid: EqBand::bypass(),
            eq_high: EqBand::bypass(),
        }
    }
}

impl MasterStage for StageEq {
    fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        for i in 0..left.len() {
            left[i] = self.eq_low.process_sample(left[i], 0);
            left[i] = self.eq_mid.process_sample(left[i], 0);
            left[i] = self.eq_high.process_sample(left[i], 0);
        }
        for i in 0..right.len() {
            right[i] = self.eq_low.process_sample(right[i], 1);
            right[i] = self.eq_mid.process_sample(right[i], 1);
            right[i] = self.eq_high.process_sample(right[i], 1);
        }
    }
    fn name(&self) -> &'static str { "eq" }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Harmonic exciter stage — wraps a stereo pair of HarmonicExciters.
pub struct StageExcite {
    pub exciter_l: HarmonicExciter,
    pub exciter_r: HarmonicExciter,
}

impl StageExcite {
    pub fn new(cutoff: f32, amount: f32) -> Self {
        let mut l = HarmonicExciter::new(cutoff, amount);
        let mut r = HarmonicExciter::new(cutoff, amount);
        l.set_sample_rate(DEFAULT_SR);
        r.set_sample_rate(DEFAULT_SR);
        StageExcite { exciter_l: l, exciter_r: r }
    }
}

impl MasterStage for StageExcite {
    fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        for sample in left.iter_mut() {
            let frame: fundsp::hacker::Frame<f32, fundsp::hacker::U1> = [*sample].into();
            let out = self.exciter_l.tick(&frame);
            *sample = out[0];
        }
        for sample in right.iter_mut() {
            let frame: fundsp::hacker::Frame<f32, fundsp::hacker::U1> = [*sample].into();
            let out = self.exciter_r.tick(&frame);
            *sample = out[0];
        }
    }
    fn name(&self) -> &'static str { "excite" }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

/// Expander stage — wraps a stereo pair of Expanders for buffer-based processing.
pub struct StageExpand {
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
}

/// Multiband compressor stage.
pub struct StageMultiband {
    pub multiband: MultibandCompressor,
}

impl StageMultiband {
    pub fn new(sample_rate: f64) -> Self {
        StageMultiband { multiband: MultibandCompressor::new(sample_rate) }
    }
}

impl MasterStage for StageMultiband {
    fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        if !self.multiband.active { return; }
        for i in 0..left.len() {
            let (l, r) = self.multiband.process_sample(left[i], right[i]);
            left[i] = l;
            right[i] = r;
        }
    }
    fn name(&self) -> &'static str { "multiband" }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

pub struct MasterBus {
    // Highpass coefficients (shared across channels)
    hp_a1: f32,
    hp_a2: f32,
    hp_b0: f32,
    hp_b1: f32,
    hp_b2: f32,
    // Highpass per-channel state [left, right]
    hp_state: [BiquadState; 2],
    // Lowpass coefficients (shared across channels)
    lp_a1: f32,
    lp_a2: f32,
    lp_b0: f32,
    lp_b1: f32,
    lp_b2: f32,
    // Lowpass per-channel state [left, right]
    lp_state: [BiquadState; 2],
    // Limiter per channel [left, right]
    limiter: BrickwallLimiter,
    // Output gain (linear). Applied before everything else in the chain.
    gain: f32,
    // User-definable chain (between filters and limiter)
    chain: Vec<Box<dyn MasterStage>>,
    pub bypass: bool,
    // For bypass loudness matching
    rms_dry: f32,
    rms_wet: f32,
}

impl MasterBus {
    pub fn new(sample_rate: f64) -> Self {
        let (hp_b0, hp_b1, hp_b2, hp_a1, hp_a2) = Self::highpass_coeffs(30.0, sample_rate);
        let (lp_b0, lp_b1, lp_b2, lp_a1, lp_a2) = Self::lowpass_coeffs(18000.0, sample_rate);
        // -0.3 dBFS ceiling ≈ 0.966
        let ceiling = 10.0_f32.powf(-0.3 / 20.0);
        let limiter = BrickwallLimiter::new(ceiling, 0.1, sample_rate);
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
            bypass: false,
            rms_dry: 0.0,
            rms_wet: 0.0,
        }
    }

    // --- Chain manipulation helpers ---

    /// Find the first stage with the given name, returning its index.
    fn find_stage(&self, name: &str) -> Option<usize> {
        self.chain.iter().position(|s| s.name() == name)
    }

    /// Replace the first stage with the given name, or append if not found.
    fn replace_or_append(&mut self, name: &str, stage: Box<dyn MasterStage>) {
        if let Some(idx) = self.find_stage(name) {
            self.chain[idx] = stage;
        } else {
            self.chain.push(stage);
        }
    }

    /// Set the entire chain from a list of stages (used by `master chain` command).
    pub fn set_chain(&mut self, stages: Vec<Box<dyn MasterStage>>) {
        self.chain = stages;
    }

    // --- Backward-compatible setter methods ---

    /// Set the master EQ curve with per-band gain in dB.
    /// `low_db`: low shelf at 120 Hz, `mid_db`: peak at 1 kHz (Q=0.7), `high_db`: high shelf at 6 kHz.
    pub fn set_curve(&mut self, low_db: f32, mid_db: f32, high_db: f32, sample_rate: f64) {
        let stage = Box::new(StageEq::new(low_db, mid_db, high_db, sample_rate));
        self.replace_or_append("eq", stage);
    }

    /// Set master EQ curve from a named preset.
    pub fn set_curve_preset(&mut self, name: &str, sample_rate: f64) -> bool {
        match name {
            "car" => { self.set_curve(-4.0, 0.0, 3.0, sample_rate); true }
            "broadcast" => { self.set_curve(-2.0, 0.0, 1.5, sample_rate); true }
            "bright" => { self.set_curve(0.0, 0.0, 4.0, sample_rate); true }
            "warm" => { self.set_curve(3.0, 0.0, -2.0, sample_rate); true }
            "flat" | "off" => {
                // Remove EQ stage if present
                if let Some(idx) = self.find_stage("eq") {
                    self.chain.remove(idx);
                }
                true
            }
            _ => false,
        }
    }

    /// Set the compressor to upward mode (boosts quiet content instead of reducing loud).
    pub fn set_compress_upward(&mut self, upward: bool) {
        if let Some(idx) = self.find_stage("compress") {
            if let Some(sc) = self.chain[idx].as_any_mut().downcast_mut::<StageCompress>() {
                sc.compressor[0].upward = upward;
                sc.compressor[1].upward = upward;
            }
        }
    }

    /// Set multiband compressor from a simple amount (0 = off, 0.3 = gentle, 1.0 = OTT-level).
    pub fn set_multiband(&mut self, amount: f32) {
        if let Some(idx) = self.find_stage("multiband") {
            if let Some(sm) = self.chain[idx].as_any_mut().downcast_mut::<StageMultiband>() {
                sm.multiband.set_amount(amount);
            }
        } else {
            let mut mb = StageMultiband::new(DEFAULT_SR);
            mb.multiband.set_amount(amount);
            self.chain.push(Box::new(mb));
        }
    }

    /// Set multiband compressor per-band amounts.
    pub fn set_multiband_per_band(&mut self, low: f32, mid: f32, high: f32) {
        if let Some(idx) = self.find_stage("multiband") {
            if let Some(sm) = self.chain[idx].as_any_mut().downcast_mut::<StageMultiband>() {
                sm.multiband.set_per_band(low, mid, high);
            }
        } else {
            let mut mb = StageMultiband::new(DEFAULT_SR);
            mb.multiband.set_per_band(low, mid, high);
            self.chain.push(Box::new(mb));
        }
    }

    /// Set the soft clipper amount.
    /// 0.0 = off (bypass), 0.5 = gentle warmth, 1.0 = moderate saturation, 2.0+ = heavy.
    pub fn set_saturate(&mut self, amount: f32) {
        let drive = if amount <= 0.0 { 0.0 } else { amount * 4.0 };
        self.replace_or_append("saturate", Box::new(StageSaturate { drive }));
    }

    /// Set master bus harmonic exciter.
    /// `cutoff`: frequency above which to excite. `amount`: 0.0-1.0.
    pub fn set_excite(&mut self, cutoff: f32, amount: f32) {
        if amount <= 0.0 {
            if let Some(idx) = self.find_stage("excite") {
                self.chain.remove(idx);
            }
        } else {
            self.replace_or_append("excite", Box::new(StageExcite::new(cutoff, amount)));
        }
    }

    /// Set the compression amount (0.0 = off, 1.0 = default, 2.0 = heavy).
    pub fn set_compress(&mut self, amount: f32, sample_rate: f64) {
        self.replace_or_append("compress", Box::new(StageCompress::new(amount, sample_rate)));
    }

    /// Set compression with explicit parameters.
    pub fn set_compress_params(&mut self, threshold: f32, ratio: f32, attack: f64, release: f64, sample_rate: f64) {
        self.replace_or_append("compress", Box::new(StageCompress::from_params(threshold, ratio, attack, release, sample_rate)));
    }

    /// Add an expander to the chain.
    pub fn add_expand(&mut self, threshold_db: f32, ratio: f32, attack: f64, release: f64) {
        self.replace_or_append("expand", Box::new(StageExpand::new(threshold_db, ratio, attack, release)));
    }

    /// Set master output gain in dB (e.g., -6.0 to reduce by 6 dB).
    /// Applied before the compressor and limiter.
    pub fn set_gain(&mut self, db: f32) {
        self.gain = 10.0_f32.powf(db / 20.0);
    }

    /// Set the limiter ceiling in dBFS (e.g., -0.3 for default, -1.0 for broadcast).
    pub fn set_ceiling(&mut self, db: f32, sample_rate: f64) {
        let ceiling = 10.0_f32.powf(db / 20.0);
        let l = BrickwallLimiter::new(ceiling, 0.1, sample_rate);
        self.limiter = l;
    }

    /// 2nd-order Butterworth highpass biquad coefficients.
    fn highpass_coeffs(freq: f64, sr: f64) -> (f32, f32, f32, f32, f32) {
        let w0 = 2.0 * std::f64::consts::PI * freq / sr;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * std::f64::consts::SQRT_2); // Q = sqrt(2)/2 for Butterworth
        let a0 = 1.0 + alpha;
        let b0 = ((1.0 + cos_w0) / 2.0 / a0) as f32;
        let b1 = (-(1.0 + cos_w0) / a0) as f32;
        let b2 = b0;
        let a1 = (-2.0 * cos_w0 / a0) as f32;
        let a2 = ((1.0 - alpha) / a0) as f32;
        (b0, b1, b2, a1, a2)
    }

    /// 2nd-order Butterworth lowpass biquad coefficients.
    fn lowpass_coeffs(freq: f64, sr: f64) -> (f32, f32, f32, f32, f32) {
        let w0 = 2.0 * std::f64::consts::PI * freq / sr;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * std::f64::consts::SQRT_2);
        let a0 = 1.0 + alpha;
        let b0 = ((1.0 - cos_w0) / 2.0 / a0) as f32;
        let b1 = ((1.0 - cos_w0) / a0) as f32;
        let b2 = b0;
        let a1 = (-2.0 * cos_w0 / a0) as f32;
        let a2 = ((1.0 - alpha) / a0) as f32;
        (b0, b1, b2, a1, a2)
    }

    /// Process a single channel through HP/LP biquad filters.
    fn process_channel(&mut self, buffer: &mut [f32], ch: usize) {
        let hp = &mut self.hp_state[ch];
        let lp = &mut self.lp_state[ch];

        for sample in buffer.iter_mut() {
            if !sample.is_finite() {
                *sample = 0.0;
                continue;
            }

            // Highpass
            let hp_out = self.hp_b0 * *sample + self.hp_b1 * hp.x1 + self.hp_b2 * hp.x2
                - self.hp_a1 * hp.y1 - self.hp_a2 * hp.y2;
            hp.x2 = hp.x1;
            hp.x1 = *sample;
            hp.y2 = hp.y1;
            hp.y1 = hp_out;

            // Lowpass
            let lp_out = self.lp_b0 * hp_out + self.lp_b1 * lp.x1 + self.lp_b2 * lp.x2
                - self.lp_a1 * lp.y1 - self.lp_a2 * lp.y2;
            lp.x2 = lp.x1;
            lp.x1 = hp_out;
            lp.y2 = lp.y1;
            lp.y1 = lp_out;

            if !lp_out.is_finite() {
                *hp = BiquadState::default();
                *lp = BiquadState::default();
                *sample = 0.0;
            } else {
                *sample = lp_out;
            }
        }
    }

    /// Process a mono buffer in-place: gain → HP → LP → chain → limiter.
    /// Uses channel 0 state. Still needed by the standalone normalization limiter path.
    pub fn process(&mut self, buffer: &mut [f32]) {
        // Apply master gain first (before all processing)
        if (self.gain - 1.0).abs() > 1e-6 {
            for sample in buffer.iter_mut() {
                *sample *= self.gain;
            }
        }

        self.process_channel(buffer, 0);

        // Run chain stages (mono: use left channel, right is a dummy zero buffer)
        let mut right_dummy = vec![0.0f32; buffer.len()];
        for stage in &mut self.chain {
            stage.process_stereo(buffer, &mut right_dummy);
        }

        // Limiter
        self.limiter.process_stereo(buffer, &mut right_dummy);
    }

    /// Process stereo buffers in-place: gain → HP → LP → chain → limiter.
    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        // Save dry buffers for bypass
        let dry_l = left.to_vec();
        let dry_r = right.to_vec();

        // Apply master gain first
        if (self.gain - 1.0).abs() > 1e-6 {
            for sample in left.iter_mut() {
                *sample *= self.gain;
            }
            for sample in right.iter_mut() {
                *sample *= self.gain;
            }
        }

        // HP + LP filters per channel
        self.process_channel(left, 0);
        self.process_channel(right, 1);

        // User chain
        for stage in &mut self.chain {
            stage.process_stereo(left, right);
        }

        // Limiter stereo
        self.limiter.process_stereo(left, right);

        // Calculate RMS for auto-gain volume matching during bypass
        let mut dry_sq = 0.0;
        let mut wet_sq = 0.0;
        for i in 0..left.len() {
            dry_sq += dry_l[i]*dry_l[i] + dry_r[i]*dry_r[i];
            wet_sq += left[i]*left[i] + right[i]*right[i];
        }
        let alpha = 0.005; // slow follower
        self.rms_dry = self.rms_dry * (1.0 - alpha) + dry_sq * alpha;
        self.rms_wet = self.rms_wet * (1.0 - alpha) + wet_sq * alpha;

        if self.bypass {
            let makeup = if self.rms_dry > 1e-6 { (self.rms_wet / self.rms_dry).sqrt() } else { 1.0 };
            // cap makeup to avoid exploding noise
            let makeup = makeup.min(10.0);
            for i in 0..left.len() {
                left[i] = dry_l[i] * makeup;
                right[i] = dry_r[i] * makeup;
            }
        }
    }

    pub fn current_gain_reduction(&self) -> f32 {
        if self.bypass { return 0.0; }
        let mut gr = 0.0;
        for stage in &self.chain {
            gr += stage.gain_reduction_db();
        }
        if self.limiter.gain_reduction > 0.0 {
            let lim_gain_db = 20.0 * (1.0 - self.limiter.gain_reduction).log10();
            gr += lim_gain_db;
        }
        gr
    }

    /// Flush limiter lookahead tail (mono — uses channel 0).
    pub fn flush(&mut self, output: &mut Vec<f32>) {
        let mut dummy = Vec::new();
        self.limiter.flush_stereo(output, &mut dummy);
    }

    /// Flush both limiter channels for stereo output.
    pub fn flush_stereo(&mut self, left: &mut Vec<f32>, right: &mut Vec<f32>) {
        self.limiter.flush_stereo(left, right);
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

// ---------------------------------------------------------------------------
// Parametric EQ (biquad-based)
// ---------------------------------------------------------------------------

/// Band type for the parametric EQ.
#[derive(Clone, Copy, Debug)]
pub enum EqBandType {
    /// Bell/peak filter: boost or cut at a center frequency with Q bandwidth.
    Peak,
    /// Low shelf: boost or cut everything below the corner frequency.
    LowShelf,
    /// High shelf: boost or cut everything above the corner frequency.
    HighShelf,
}

/// Single-band parametric EQ using a biquad filter.
///
/// Supports three band types:
/// - **Peak** (bell): `eq(freq, gain_db, q)` — boost/cut at freq with bandwidth Q
/// - **Low shelf**: `eq(freq, gain_db, "low")` — boost/cut below freq
/// - **High shelf**: `eq(freq, gain_db, "high")` — boost/cut above freq
///
/// Uses the Audio EQ Cookbook biquad formulas (Robert Bristow-Johnson).
/// 1 input, 1 output.
#[derive(Clone)]
pub struct ParametricEQ {
    // Biquad state
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    // Stored for recalculation on sample rate change
    freq: f32,
    gain_db: f32,
    q: f32,
    band_type: EqBandType,
    sample_rate: f64,
}

impl ParametricEQ {
    pub fn new(freq: f32, gain_db: f32, q: f32, band_type: EqBandType) -> Self {
        let mut eq = ParametricEQ {
            b0: 1.0, b1: 0.0, b2: 0.0,
            a1: 0.0, a2: 0.0,
            x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0,
            freq,
            gain_db,
            q: q.max(0.1),
            band_type,
            sample_rate: 0.0,
        };
        eq.set_sample_rate(DEFAULT_SR);
        eq
    }

    fn recalc(&mut self) {
        let sr = self.sample_rate as f32;
        let w0 = 2.0 * std::f32::consts::PI * self.freq / sr;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let a_lin = 10.0_f32.powf(self.gain_db / 40.0); // sqrt of linear gain

        match self.band_type {
            EqBandType::Peak => {
                let alpha = sin_w0 / (2.0 * self.q);
                let a0 = 1.0 + alpha / a_lin;
                self.b0 = (1.0 + alpha * a_lin) / a0;
                self.b1 = (-2.0 * cos_w0) / a0;
                self.b2 = (1.0 - alpha * a_lin) / a0;
                self.a1 = (-2.0 * cos_w0) / a0;
                self.a2 = (1.0 - alpha / a_lin) / a0;
            }
            EqBandType::LowShelf => {
                let alpha = sin_w0 / (2.0 * self.q);
                let two_sqrt_a_alpha = 2.0 * a_lin.sqrt() * alpha;
                let a0 = (a_lin + 1.0) + (a_lin - 1.0) * cos_w0 + two_sqrt_a_alpha;
                self.b0 = (a_lin * ((a_lin + 1.0) - (a_lin - 1.0) * cos_w0 + two_sqrt_a_alpha)) / a0;
                self.b1 = (2.0 * a_lin * ((a_lin - 1.0) - (a_lin + 1.0) * cos_w0)) / a0;
                self.b2 = (a_lin * ((a_lin + 1.0) - (a_lin - 1.0) * cos_w0 - two_sqrt_a_alpha)) / a0;
                self.a1 = (-2.0 * ((a_lin - 1.0) + (a_lin + 1.0) * cos_w0)) / a0;
                self.a2 = ((a_lin + 1.0) + (a_lin - 1.0) * cos_w0 - two_sqrt_a_alpha) / a0;
            }
            EqBandType::HighShelf => {
                let alpha = sin_w0 / (2.0 * self.q);
                let two_sqrt_a_alpha = 2.0 * a_lin.sqrt() * alpha;
                let a0 = (a_lin + 1.0) - (a_lin - 1.0) * cos_w0 + two_sqrt_a_alpha;
                self.b0 = (a_lin * ((a_lin + 1.0) + (a_lin - 1.0) * cos_w0 + two_sqrt_a_alpha)) / a0;
                self.b1 = (-2.0 * a_lin * ((a_lin - 1.0) + (a_lin + 1.0) * cos_w0)) / a0;
                self.b2 = (a_lin * ((a_lin + 1.0) + (a_lin - 1.0) * cos_w0 - two_sqrt_a_alpha)) / a0;
                self.a1 = (2.0 * ((a_lin - 1.0) - (a_lin + 1.0) * cos_w0)) / a0;
                self.a2 = ((a_lin + 1.0) - (a_lin - 1.0) * cos_w0 - two_sqrt_a_alpha) / a0;
            }
        }
    }
}

impl AudioNode for ParametricEQ {
    const ID: u64 = 1009;
    type Inputs = U1;
    type Outputs = U1;

    fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
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
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        [y].into()
    }
}
