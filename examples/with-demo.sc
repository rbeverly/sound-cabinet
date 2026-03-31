// Demonstrates the `with` clause for voice substitution
// The same pattern plays with different instruments

bpm 120

import voices/instruments.sc

// Mix levels — instruments from instruments.sc
normalize rhodes 0.3
normalize kalimba 0.3

// Two styles of kick, snare, and hat
voice kick_a = (0.7 * sine(A1) + 0.3 * sine(A2) >> decay(30) + 0.15 * (noise() >> lowpass(200, 0.5) >> decay(60))) >> decay(10) >> compress(-12, 4, 0.001, 0.05)
voice kick_b = 0.6 * sine(G1) >> decay(8) >> distort(1.5)
voice snare_a = (0.15 * sine(D3) >> decay(25) + 0.2 * noise() >> bandpass(3000, 1.5)) >> decay(18) >> compress(-15, 3, 0.001, 0.08)
voice snare_b = 0.15 * noise() >> lowpass(3000, 0.8) >> decay(20)
voice hat_a = 0.08 * noise() >> highpass(8000, 0.5) >> decay(30)
voice hat_b = 0.06 * noise() >> bandpass(6000, 2.0) >> decay(25)

// Synth lead
instrument synth_lead = (0.3 * saw(freq) + 0.2 * pulse(freq, 0.3)) >> lowpass(freq * 4, 0.7) >> decay(6)

// Mix levels — local voices and instruments
normalize kick_a 0.15
normalize kick_b 0.15
normalize snare_a 0.18
normalize snare_b 0.18
normalize synth_lead 0.35

master compress 1.0

// A drum pattern using placeholder names
pattern beat = 4 beats
  at 0 play kick for 0.5 beats
  at 1 play snare for 0.25 beats
  at 2 play kick for 0.5 beats
  at 2.5 play hat for 0.25 beats
  at 3 play snare for 0.25 beats
  at 3.5 play hat for 0.25 beats

// A melodic pattern using a placeholder name
pattern melody_line = 4 beats
  at 0 play mel(C4) >> swell(0.05, 0.3) for 1.0 beats
  at 1 play mel(E4) >> swell(0.05, 0.3) for 0.5 beats
  at 1.5 play mel(G4) >> swell(0.05, 0.3) for 0.5 beats
  at 2 play mel(A4) >> swell(0.05, 0.3) for 1.0 beats
  at 3 play mel(G4) >> swell(0.05, 0.3) for 1.0 beats

// Electronic version
section electronic = 16 beats with {kick = kick_a, snare = snare_a, hat = hat_a, mel = synth_lead}
  repeat beat every 4 beats
  repeat melody_line every 4 beats

// Organic version — same patterns, different instruments
section organic = 16 beats with {kick = kick_b, snare = snare_b, hat = hat_b, mel = kalimba}
  repeat beat every 4 beats
  repeat melody_line every 4 beats

// Hybrid: electronic drums, but override melody to rhodes per-entry
section hybrid = 16 beats with {kick = kick_a, snare = snare_a, hat = hat_a, mel = kalimba}
  repeat beat every 4 beats
  repeat melody_line every 4 beats with {mel = rhodes}

play electronic
play organic
play hybrid
