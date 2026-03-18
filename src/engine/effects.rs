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
