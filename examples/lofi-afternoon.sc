// lofi-afternoon.sc
// A ~2 minute lo-fi composition with arrangement arc.

import voices/lofi-kit.sc

// Mix levels
normalize chord_pad 0.2
normalize bass 0.3
normalize mel 0.35
normalize pad 0.2
normalize kick 0.15
normalize snare 0.2
normalize ghost_snare 0.15

master compress 1.0
master gain -3

bpm 75
humanize 6

// -- Patterns --

pattern boom_bap = 4 beats
  at 0 play kick for 0.5 beats
  at 1 play snare for 0.25 beats
  at 2 play kick for 0.5 beats
  at 3 play snare for 0.25 beats

// Busier variation — swung offbeats for lazy feel
pattern boom_bap_busy = 4 beats swing 0.65
  at 0   play kick for 0.5 beats
  at 1   play snare for 0.25 beats
  at 1.5 play ghost_snare for 0.15 beats
  at 2   play kick for 0.5 beats
  at 2.5 play hat for 0.15 beats
  at 3   play snare for 0.25 beats

// Standard hats
pattern hats = 4 beats
  at 0 play hat for 0.2 beats
  at 1 play hat for 0.2 beats
  at 2 play hat for 0.2 beats
  at 3 play hat for 0.2 beats

// Hats with a pickup on the "and" of 4 — subtle push into next bar
pattern hats_push = 4 beats
  at 0   play hat for 0.2 beats
  at 1   play hat for 0.2 beats
  at 2   play hat for 0.2 beats
  at 3   play hat for 0.2 beats
  at 3.5 play hat for 0.1 beats

// Chord progression: Cm7 → Abmaj7 → Fm7 → Bbmaj7
// Bbmaj7 replaces Gdom7 — dominant 7th tritone was clashing with the melody
pattern chords = 16 beats
  at 0  play chord_pad(Cm7) for 4 beats
  at 4  play chord_pad(Abmaj7) for 4 beats
  at 8  play chord_pad(Fm7) for 4 beats
  at 12 play chord_pad(Bbmaj7) for 4 beats

// Bass follows chord roots
pattern bass_line = 16 beats
  at 0  play bass(C2) for 4 beats
  at 4  play bass(Ab1) for 4 beats
  at 8  play bass(F1) for 4 beats
  at 12 play bass(Bb1) for 4 beats

// Vinyl texture — constant hiss + surface imperfections cycling with the record
// 33 RPM = 1.82s per rotation = 2.27 beats at 75 bpm
// 3 imperfections on the surface: a pop, a click, and a scratch
// They repeat every 2.27 beats as the record comes around
pattern vinyl = 16 beats
  at 0    play hiss for 16 beats
  at 0.0  play pop for 0.1 beats
  at 0.15 play click for 0.05 beats
  at 1.2  play scratch for 0.15 beats
  at 2.27 play pop for 0.1 beats
  at 2.42 play click for 0.05 beats
  at 3.47 play scratch for 0.15 beats
  at 4.54 play pop for 0.1 beats
  at 4.69 play click for 0.05 beats
  at 5.74 play scratch for 0.15 beats
  at 6.81 play pop for 0.1 beats
  at 6.96 play click for 0.05 beats
  at 8.01 play scratch for 0.15 beats
  at 9.08 play pop for 0.1 beats
  at 9.23 play click for 0.05 beats
  at 10.28 play scratch for 0.15 beats
  at 11.35 play pop for 0.1 beats
  at 11.50 play click for 0.05 beats
  at 12.55 play scratch for 0.15 beats
  at 13.62 play pop for 0.1 beats
  at 13.77 play click for 0.05 beats
  at 14.82 play scratch for 0.15 beats

pattern melody_a = 16 beats
  at 0   play mel(C5)  for 1 beat
  at 1.5 play mel(Eb5) for 0.5 beats
  at 2   play mel(G4)  for 2 beats
  at 4   play mel(Bb4) for 1.5 beats
  at 6   play mel(G4)  for 1 beat
  at 7.5 play mel(Eb5) for 0.5 beats
  at 8   play mel(C5)  for 1 beat
  at 9   play mel(Eb5) for 1 beat
  at 10  play mel(F5)  for 1 beat
  at 11  play mel(Eb5) for 1 beat
  at 12  play mel(G4)  for 2 beats
  at 14  play mel(Bb4) for 1 beat
  at 15  play mel(C5)  for 1 beat

pattern melody_b = 16 beats
  at 0   play mel(Eb5) for 1 beat
  at 1   play mel(C5)  for 1.5 beats
  at 3   play mel(G4)  for 1 beat
  at 4   play mel(Bb4) for 1 beat
  at 5   play mel(C5)  for 1 beat
  at 6.5 play mel(Eb5) for 1.5 beats
  at 8   play mel(F5)  for 1 beat
  at 9   play mel(Eb5) for 0.5 beats
  at 9.5 play mel(C5)  for 1.5 beats
  at 11  play mel(G4)  for 1 beat
  at 12  play mel(C5)  for 1.5 beats
  at 14  play mel(Eb5) for 1 beat
  at 15  play mel(G4)  for 1 beat

// Climax melody — more active, higher register
pattern melody_climax = 16 beats
  at 0   play mel(F5)  for 0.5 beats
  at 0.5 play mel(Eb5) for 0.5 beats
  at 1   play mel(C5)  for 1 beat
  at 2   play mel(Eb5) for 1 beat
  at 3   play mel(F5)  for 1 beat
  at 4   play mel(Eb5) for 0.5 beats
  at 4.5 play mel(C5)  for 0.5 beats
  at 5   play mel(Bb4) for 1 beat
  at 6   play mel(C5)  for 1 beat
  at 7   play mel(Eb5) for 1 beat
  at 8   play mel(F5)  for 1.5 beats
  at 10  play mel(Eb5) for 1 beat
  at 11  play mel(F5)  for 0.5 beats
  at 11.5 play mel(Eb5) for 0.5 beats
  at 12  play mel(C5)  for 1.5 beats
  at 14  play mel(Eb5) for 1 beat
  at 15  play mel(F5)  for 1 beat

pattern sparse_melody = 16 beats
  at 0   play mel(C5)  for 2 beats
  at 4   play mel(Eb5) for 2 beats
  at 8   play mel(G4)  for 4 beats

// -- Sections --

// Just chords, bass, crackle — no drums, no melody
section intro = 16 beats
  play chords
  play bass_line
  play vinyl

// Drums enter, no melody yet — lets the beat land
section drums_only = 16 beats
  play chords
  play bass_line
  play vinyl
  repeat boom_bap every 4 beats
  repeat hats every 4 beats

// Full groove — drums + melody
section groove_a = 16 beats
  play chords
  play bass_line
  play vinyl
  repeat boom_bap every 4 beats
  repeat hats every 4 beats
  play melody_a

section groove_b = 16 beats
  play chords
  play bass_line
  play vinyl
  repeat boom_bap every 4 beats
  repeat hats every 4 beats
  play melody_b

// Pad swell — airy texture underneath the climax
pattern pad_layer = 16 beats
  at 0 play pad(C3) >> swell(2.0, 2.0) for 8 beats
  at 0 play pad(Eb3) >> swell(2.0, 2.0) for 8 beats
  at 0 play pad(G3) >> swell(2.0, 2.0) for 8 beats
  at 8 play pad(Ab3) >> swell(2.0, 2.0) for 8 beats
  at 8 play pad(C4) >> swell(2.0, 2.0) for 8 beats
  at 8 play pad(Eb4) >> swell(2.0, 2.0) for 8 beats

// Climax — ghost notes, hat pickups, and pad layer add energy
section climax = 16 beats
  play chords
  play bass_line
  play vinyl
  play pad_layer
  repeat boom_bap_busy every 4 beats
  repeat hats_push every 4 beats
  play melody_climax

// Wind down — back to simple drums, sparse melody
section wind_down = 16 beats
  play chords
  play bass_line
  play vinyl
  repeat boom_bap every 4 beats
  play sparse_melody

// Outro — no drums
section outro = 16 beats
  play chords
  play bass_line
  play vinyl
  play sparse_melody

// Tail — hiss trailing off with one last pop
pattern tail_hiss = 4 beats
  at 0   play hiss for 4 beats
  at 1.3 play pop for 0.1 beats

section tail = 4 beats
  play tail_hiss

// -- Arrangement --
// Arc: intro → drums land → add melody → build → climax → wind down → outro → tail

play intro
play drums_only
repeat 3 {
  pick [groove_a, groove_b]
}
repeat 2 {
  play climax
}
play wind_down
play outro
play tail
