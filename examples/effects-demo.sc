// effects-demo.sc — Showcase of new DSL features
// Note names, LFO, distortion, vibrato, chorus, and arpeggiator

bpm 100

// --- Note names: A4, Bb3, C#5, Fs4 all resolve to Hz at parse time ---

// Warm pad with vibrato
voice warm_pad = (0.3 * saw(C4) + 0.3 * saw(E4) + 0.3 * saw(G4)) >> lowpass(1200, 0.7) >> vibrato(4.0, 15.0)

// Lead with LFO tremolo
voice lead = 0.4 * triangle(G5) >> lfo(6.0, 0.4)

// Dirty bass with distortion
voice dirty_bass = 0.5 * saw(C2) >> lowpass(400, 1.2) >> distort(4.0)

// Shimmery texture with chorus
voice shimmer = 0.2 * triangle(E5) >> chorus(0.015, 0.005, 0.3)

// Simple kick and hat
voice kick = (0.7 * sine(A1) + 0.5 * sine(B0)) >> decay(12)
voice hat = 0.1 * noise() >> highpass(6000, 1.0) >> decay(25)

// --- Patterns ---

pattern beat = 4 beats
  at 0 play kick for 0.5 beats
  at 1 play hat for 0.25 beats
  at 2 play kick for 0.5 beats
  at 3 play hat for 0.25 beats
  at 3.5 play hat for 0.25 beats

// Arpeggiator: cycles through C minor triad at 16th notes
pattern arp_pattern = 4 beats
  at 0 play arp(C4, Eb4, G4, Bb4, 4) for 4 beats

pattern lead_phrase = 4 beats
  at 0 play lead >> swell(0.5, 1.0) for 3 beats

pattern pad_bed = 8 beats
  at 0 play warm_pad >> swell(2.0, 2.0) for 8 beats

pattern bass_line = 4 beats
  at 0 play dirty_bass for 2 beats
  at 2.5 play dirty_bass for 1.5 beats

pattern shimmer_layer = 8 beats
  at 0 play shimmer >> swell(3.0, 2.0) for 8 beats

// --- Arrangement ---

section intro = 8 beats
  repeat beat every 4 beats
  play pad_bed

section main = 8 beats
  repeat beat every 4 beats
  repeat bass_line every 4 beats
  repeat arp_pattern every 4 beats
  play pad_bed

section outro = 8 beats
  repeat beat every 4 beats
  play shimmer_layer

play intro
play main
play main
play outro
