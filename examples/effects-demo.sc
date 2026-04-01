// effects-demo.sc — Showcase of DSL features
// Note names, effects, fx chains, chords, and arpeggiator

bpm 100

// --- Effect chains (reusable pipelines) ---

fx hall = reverb(0.8, 0.4, 0.3)
fx echo = delay(0.45, 0.4, 0.35)
fx wide = chorus(0.015, 0.005, 0.3) >> reverb(0.9, 0.6, 0.4)

// --- Instrument: freq-relative filter tracking across all notes ---

instrument keys = (0.4 * saw(freq) >> lowpass(freq * 4, 0.7) >> decay(8)) + (1.5 * saw(freq) + 0.3 * saw(freq * 2)) >> lowpass(freq * 1.2, 0.6) >> chorus(0.016, 0.006, 0.1) >> decay(2.5) >> hall

// Mix levels (instruments only — voices normalized after definition below)
normalize keys 0.3

master compress 1.0
master gain -3

// --- Voices (using fx chains) ---

// Warm pad with vibrato and hall reverb
voice warm_pad = chord(C:maj) >> lowpass(1200, 0.7) >> vibrato(4.0, 15.0) >> hall

// Lead with dotted-eighth delay echoes
voice lead = 0.4 * triangle(G5) >> lfo(6.0, 0.4) >> echo

// Dirty bass with distortion and compression — evens out dynamics
voice dirty_bass = 0.5 * saw(C2) >> lowpass(400, 1.2) >> distort(4.0) >> compress(-12, 4, 0.005, 0.1)

// Filter sweep: lowpass opens from 200 to 4000 Hz over the event duration
voice sweep_pad = (0.4 * saw(C3) + 0.3 * saw(G3)) >> lowpass(200 -> 4000, 0.7)

// Pulse wave — nasal, reedy character. 10% width = thin, 50% = square
voice pulse_bass = 0.4 * pulse(C2, 0.15) >> lowpass(600, 0.8)

// PWM sweep — pulse width modulates from thin to fat over the event duration
voice pwm_pad = 0.3 * pulse(C3, 0.1 -> 0.9) >> lowpass(1500, 0.6) >> chorus(0.014, 0.005, 0.2)

// Shimmery texture with chorus and reverb
voice shimmer = 0.2 * triangle(E5) >> wide

// Arp voice using the instrument — freq gets substituted per note
voice pluck = 0.3 * saw(0) >> lowpass(2000, 0.8) >> decay(10)

// Simple kick and hat
voice kick = (0.7 * sine(A1) + 0.5 * sine(B0)) >> decay(12)
voice hat = 0.1 * noise() >> highpass(6000, 1.0) >> decay(25)
normalize kick 0.15

// --- Patterns ---

pattern beat = 4 beats
  at 0 play kick for 0.5 beats
  at 1 play hat for 0.25 beats
  at 2 play kick for 0.5 beats
  at 3 play hat for 0.25 beats
  at 3.5 play hat for 0.25 beats

// Arpeggiator with chord shorthand — C:m7 expands to C4, Eb4, G4, Bb4
pattern arp_pattern = 4 beats
  at 0 play pluck >> arp(C:m7, 4) >> lowpass(1500, 0.6) >> delay(0.3, 0.35, 0.3) for 4 beats

// Instrument used directly — one definition, any note
pattern keys_phrase = 4 beats
  at 0 play keys(C4) for 2 beats
  at 2 play keys(Eb4) for 1 beat
  at 3 play keys(G4) for 1 beat

pattern lead_phrase = 4 beats
  at 0 play lead >> swell(0.5, 1.0) for 3 beats

pattern pad_bed = 8 beats
  at 0 play warm_pad >> swell(2.0, 2.0) for 8 beats

pattern bass_line = 4 beats
  at 0 play dirty_bass for 2 beats
  at 2.5 play dirty_bass for 1.5 beats

pattern shimmer_layer = 8 beats
  at 0 play shimmer >> swell(3.0, 2.0) for 8 beats

// Filter sweep riser — opens up over 8 beats
pattern sweep_riser = 8 beats
  at 0 play sweep_pad >> swell(1.0, 1.0) for 8 beats

// Pulse bass pattern
pattern pulse_line = 4 beats
  at 0 play pulse_bass for 2 beats
  at 2.5 play pulse_bass for 1 beat

// PWM sweep — the signature 80s synth pad
pattern pwm_layer = 8 beats
  at 0 play pwm_pad >> swell(1.5, 2.0) for 8 beats

// --- Arrangement ---

section intro = 8 beats
  repeat beat every 4 beats
  play pad_bed

section main = 8 beats
  repeat beat every 4 beats
  repeat bass_line every 4 beats
  repeat arp_pattern every 4 beats
  play pad_bed

section bridge = 8 beats
  repeat beat every 4 beats
  repeat keys_phrase every 4 beats
  play sweep_riser

section pulse_section = 8 beats
  repeat beat every 4 beats
  repeat pulse_line every 4 beats
  repeat arp_pattern every 4 beats
  play pwm_layer

section outro = 8 beats
  repeat beat every 4 beats
  play shimmer_layer

play intro
play main
play main
play bridge
play pulse_section
play pulse_section
play main
play outro
