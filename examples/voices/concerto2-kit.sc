// concerto2-kit.sc — Piano instrument for Rachmaninoff Concerto No. 2
//
// Single instrument definition replaces 85 per-note voice definitions.
// freq-relative filter tracking ensures the timbre scales naturally:
//   - Hammer attack LP: freq*3 + 1000 (bright transient with bass floor)
//   - String body LP: freq*2 + 200 (warm body, bass gets enough harmonics)
//   - Equal-loudness compensation: (100/freq + 0.5) boosts bass, tapers treble
//     F1=2.8x, C3=1.3x, C4=0.9x, C5=0.7x — matches Fletcher-Munson curve
//   - Chorus for sympathetic string resonance
//   - Reverb for concert hall depth

fx hall = reverb(0.7, 0.35, 0.35)

instrument piano = (200 / freq) * (((0.5 * saw(freq) >> lowpass(freq * 3 + 1000, 0.7) >> decay(8)) + (1.5 * saw(freq) + 0.4 * saw(freq * 2) + 0.8 * sine(freq)) >> lowpass(freq * 2 + 200, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.8) >> hall)
