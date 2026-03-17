// lofi-afternoon-v2.sc
// Same composition as lofi-afternoon.sc, but using patterns, sections, and repeat.

import voices/lofi-kit.sc

bpm 75

// -- Patterns --

pattern boom_bap = 4 beats
  at 0 play kick for 0.5 beats
  at 1 play snare for 0.25 beats
  at 2 play kick for 0.5 beats
  at 3 play snare for 0.25 beats

pattern hats = 4 beats
  at 0 play hat for 0.2 beats
  at 1 play hat for 0.2 beats
  at 2 play hat for 0.2 beats
  at 3 play hat for 0.2 beats

pattern chords = 16 beats
  at 0  play chord1 for 4 beats
  at 4  play chord2 for 4 beats
  at 8  play chord3 for 4 beats
  at 12 play chord4 for 4 beats

pattern bass = 16 beats
  at 0  play bass_c  for 4 beats
  at 4  play bass_ab for 4 beats
  at 8  play bass_f  for 4 beats
  at 12 play bass_g  for 4 beats

pattern crackle_loop = 16 beats
  at 0 play crackle for 16 beats

pattern melody_a = 16 beats
  at 0   play mel_c5  for 1 beat
  at 1.5 play mel_eb5 for 0.5 beats
  at 2   play mel_g4  for 2 beats
  at 4   play mel_bb4 for 1.5 beats
  at 6   play mel_g4  for 1 beat
  at 7.5 play mel_eb5 for 0.5 beats
  at 8   play mel_c5  for 1 beat
  at 9   play mel_eb5 for 1 beat
  at 10  play mel_f5  for 1 beat
  at 11  play mel_eb5 for 1 beat
  at 12  play mel_g4  for 2 beats
  at 14  play mel_bb4 for 1 beat
  at 15  play mel_c5  for 1 beat

pattern melody_b = 16 beats
  at 0   play mel_eb5 for 1 beat
  at 1   play mel_c5  for 1.5 beats
  at 3   play mel_g4  for 1 beat
  at 4   play mel_bb4 for 1 beat
  at 5   play mel_c5  for 1 beat
  at 6.5 play mel_eb5 for 1.5 beats
  at 8   play mel_f5  for 1 beat
  at 9   play mel_eb5 for 0.5 beats
  at 9.5 play mel_c5  for 1.5 beats
  at 11  play mel_g4  for 1 beat
  at 12  play mel_c5  for 1.5 beats
  at 14  play mel_eb5 for 1 beat
  at 15  play mel_g4  for 1 beat

pattern melody_c = 16 beats
  at 0   play mel_g4  for 1 beat
  at 1   play mel_bb4 for 1 beat
  at 2   play mel_c5  for 2 beats
  at 4   play mel_eb5 for 2 beats
  at 6   play mel_c5  for 1 beat
  at 7   play mel_bb4 for 1 beat
  at 8   play mel_c5  for 0.5 beats
  at 8.5 play mel_eb5 for 0.5 beats
  at 9   play mel_f5  for 1 beat
  at 10  play mel_eb5 for 2 beats
  at 12  play mel_c5  for 2 beats
  at 14  play mel_g4  for 2 beats

pattern sparse_melody = 16 beats
  at 0   play mel_c5  for 2 beats
  at 4   play mel_eb5 for 2 beats
  at 8   play mel_g4  for 4 beats

// -- Sections --

section intro = 16 beats
  play chords
  play bass
  play crackle_loop

section groove_a = 16 beats
  play chords
  play bass
  play crackle_loop
  repeat boom_bap every 4 beats
  repeat hats every 4 beats
  play melody_a

section groove_b = 16 beats
  play chords
  play bass
  play crackle_loop
  repeat boom_bap every 4 beats
  repeat hats every 4 beats
  play melody_b

section groove_c = 16 beats
  play chords
  play bass
  play crackle_loop
  repeat boom_bap every 4 beats
  repeat hats every 4 beats
  play melody_c

section outro = 16 beats
  play chords
  play bass
  play crackle_loop
  play sparse_melody

// -- Arrangement --

play intro
repeat 8 {
  pick [groove_a:2, groove_b:2, groove_c:1]
}
play outro
