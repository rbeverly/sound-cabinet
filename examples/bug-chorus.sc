// bug-chorus.sc — Gamma entrainment as emergent insect chorus
// ~1 minute at 72 BPM
//
// Three "bug" voices with slightly detuned carriers and repetition
// rates. The carrier detuning creates alpha-rate (10 Hz) shimmer
// through acoustic beating. The repetition-rate detuning makes the
// click trains drift in and out of phase — like real cicada choruses
// where thousands of individuals aren't perfectly synchronized.
//
// Voice 1: center bug  — 10000 Hz carrier, 40.00 Hz rate
// Voice 2: bright bug  — 10010 Hz carrier, 40.05 Hz rate (+10 Hz = alpha beat)
// Voice 3: dark bug    — 9994 Hz carrier, 39.95 Hz rate  (-6 Hz ≈ theta beat vs center)
//
// The combined effect: gamma entrainment (40 Hz) with embedded
// alpha shimmer from the carrier beating, plus slow organic phasing
// from the repetition rate drift.

// --- Bug voices ---

// Center bug: the anchor — 10 kHz carrier modulated at exactly 40 Hz
// Using triangle for smooth amplitude envelope (no discontinuity artifacts).
// Triangle is bipolar (-1 to +1) so effective peak is ~15% of coefficient.
voice bug_center = (1.5 * sine(10000) >> highpass(6000, 1.0)) * triangle(40)

// Bright bug: 10 Hz above center carrier, hair faster repetition
// Carrier beat with center = 10010 - 10000 = 10 Hz (alpha)
// Rep rate drift = 40.05 - 40.00 = 0.05 Hz (one full phase cycle every 20 sec)
voice bug_bright = (1.3 * sine(10010) >> highpass(6000, 1.0)) * triangle(40.05)

// Dark bug: 6 Hz below center carrier, hair slower repetition
// Carrier beat with center = 10000 - 9994 = 6 Hz (theta)
// Rep rate drift = 40.00 - 39.95 = 0.05 Hz (drifts opposite to bright)
voice bug_dark = (1.2 * sine(9994) >> highpass(6000, 1.0)) * triangle(39.95)

// Breath: filtered noise to fill the gaps, like nighttime ambience
voice night_air = 0.02 * noise() >> highpass(2000, 0.5) >> lowpass(7000, 0.4)

bpm 72

// --- Single continuous section ---
// Each voice spans the entire track as one event so oscillator phases
// never reset. Staggered swell envelopes create the build-up/fade shape.
//
// Layout (96 beats @ 72 BPM ≈ 80s):
//   0-16   solo: center fades in
//  16-32   duo:  bright fades in, center holds
//  32-48   trio: dark fades in, others hold
//  48-80   full: all steady — listen to the shimmer
//  80-96   fade: staggered fade-out

section main
  // Center: fades in over first 8s, fades out over last 10s
  at 0 play bug_center >> swell(8.0, 10.0) for 96 beats
  // Bright: enters at beat 16, fades in 6s, fades out over last 7s
  at 32 play bug_bright >> swell(6.0, 7.0) for 64 beats
  // Dark: enters at beat 32, fades in 5s, fades out over last 4s (drops first)
  at 64 play bug_dark >> swell(5.0, 4.0) for 32 beats

// --- Arrangement ---
// 96 beats @ 72 BPM ≈ 80s ≈ ~1:20
play main
