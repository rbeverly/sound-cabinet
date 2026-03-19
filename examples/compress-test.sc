// compress-test.sc — A/B comparison of compression
// First 8 beats: no compression. Next 8 beats: compressed.
// Listen for: tighter dynamics, more even volume, punchier attack

bpm 120

// Uncompressed voices
voice bass_dry = 0.6 * saw(C2) >> lowpass(500, 1.0) >> distort(3.0)
voice pad_dry = (0.5 * saw(C3) + 0.4 * saw(G3) + 0.3 * saw(E4)) >> lowpass(2000, 0.7)
voice hit_dry = (0.9 * sine(C2) + 0.7 * noise()) >> lowpass(3000, 0.8) >> decay(8)

// Same voices but compressed
voice bass_wet = 0.6 * saw(C2) >> lowpass(500, 1.0) >> distort(3.0) >> compress(-15, 6, 0.003, 0.08)
voice pad_wet = (0.5 * saw(C3) + 0.4 * saw(G3) + 0.3 * saw(E4)) >> lowpass(2000, 0.7) >> compress(-18, 4, 0.01, 0.2)
voice hit_wet = (0.9 * sine(C2) + 0.7 * noise()) >> lowpass(3000, 0.8) >> decay(8) >> compress(-10, 8, 0.001, 0.05)

// --- DRY (no compression) ---

pattern dry_section = 8 beats
  at 0 play bass_dry for 1.5 beats
  at 0 play pad_dry >> swell(0.5, 0.5) for 4 beats
  at 0 play hit_dry for 1 beat
  at 2 play bass_dry for 1.5 beats
  at 2 play hit_dry for 1 beat
  at 4 play bass_dry for 1.5 beats
  at 4 play pad_dry >> swell(0.5, 0.5) for 4 beats
  at 4 play hit_dry for 1 beat
  at 6 play bass_dry for 1.5 beats
  at 6 play hit_dry for 1 beat

// --- WET (compressed) ---

pattern wet_section = 8 beats
  at 0 play bass_wet for 1.5 beats
  at 0 play pad_wet >> swell(0.5, 0.5) for 4 beats
  at 0 play hit_wet for 1 beat
  at 2 play bass_wet for 1.5 beats
  at 2 play hit_wet for 1 beat
  at 4 play bass_wet for 1.5 beats
  at 4 play pad_wet >> swell(0.5, 0.5) for 4 beats
  at 4 play hit_wet for 1 beat
  at 6 play bass_wet for 1.5 beats
  at 6 play hit_wet for 1 beat

// 2 beats of silence between sections so you can hear the transition
section dry = 10 beats
  play dry_section

section wet = 10 beats
  play wet_section

// DRY - SILENCE - WET - SILENCE - DRY - WET
play dry
play wet
play dry
play wet
