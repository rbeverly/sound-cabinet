// concerto2-kit.sc — Piano instrument for Rachmaninoff Concerto No. 2
//
// Single instrument definition replaces 85 per-note voice definitions.
// freq-relative filter tracking ensures the timbre scales naturally:
//   - Hammer attack LP tracks freq*6 (bright transient, scales with pitch)
//   - String body LP tracks freq*2 (warm body, darker for bass, opens for treble)
//   - Chorus for sympathetic string resonance
//   - Reverb for concert hall ambience

fx hall = reverb(0.6, 0.4, 0.3)

instrument piano = ((0.45 * saw(freq) >> lowpass(freq * 6, 0.7) >> decay(8)) + (1.8 * saw(freq) + 0.35 * saw(freq * 2)) >> lowpass(freq * 2, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> hall
