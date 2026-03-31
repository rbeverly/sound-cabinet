// black-glass.sc — Downtempo electronic, organic textures
// Patient build. Layered percussion. Deep bass. Drifting melody.
// ~3 minutes. Key of D minor.

import voices/organic-kit.sc

// Mix levels
normalize pluck 0.1
normalize thumb 0.3
normalize shimmer 0.2
normalize deep 0.2
normalize atmos 0.2
normalize silk 0.2
normalize dk 0.085
normalize sn 0.028
normalize rim 0.028
normalize ht 0.02
normalize oht 0.03
normalize hum 0.000966
normalize air 0.00033

master compress 0.5
master ceiling -1.0
master gain -3

bpm 98
humanize 6

// ============================================================
// PATTERNS
// ============================================================

// -- Percussion layers --

// Core beat — sparse kick and rim
pattern beat_sparse = 8 beats swing 0.55
  at 0   play dk for 0.5 beats
  at 3   play rim for 0.2 beats
  at 4   play dk for 0.5 beats
  at 6.5 play rim for 0.2 beats

// Fuller beat — kick, snare, rim interplay
pattern beat_full = 8 beats swing 0.55
  at 0   play dk for 0.5 beats
  at 2.5 play sn for 0.25 beats
  at 3   play rim for 0.2 beats
  at 4   play dk for 0.5 beats
  at 5.5 play rim for 0.2 beats
  at 6.5 play sn for 0.25 beats

// Shaker pattern — steady eighth notes, swung
pattern shakers = 4 beats swing 0.57 humanize 8
  at 0   play shk for 0.3 beats
  at 0.5 play shk for 0.3 beats
  at 1   play shk for 0.3 beats
  at 1.5 play shk for 0.3 beats
  at 2   play shk for 0.3 beats
  at 2.5 play shk for 0.3 beats
  at 3   play shk for 0.3 beats
  at 3.5 play shk for 0.3 beats

// Hat pattern — syncopated, opens on the and of 3
pattern hats = 8 beats swing 0.55 humanize 5
  at 0.5 play ht for 0.15 beats
  at 2   play ht for 0.15 beats
  at 3.5 play ht for 0.15 beats
  at 5   play oht for 0.3 beats
  at 6   play ht for 0.15 beats
  at 7.5 play ht for 0.15 beats

// -- Harmonic layers --

// Chord progression: Dm9 → Bbmaj7 → Gm7 → Am7
// Voiced as pads — slow swell, long sustain
pattern chords_a = 32 beats
  at 0  play 0.15 * atmos(D3) >> swell(3.0, 2.0) for 8 beats
  at 0  play 0.12 * atmos(F3) >> swell(3.0, 2.0) for 8 beats
  at 0  play 0.10 * atmos(A3) >> swell(3.0, 2.0) for 8 beats
  at 0  play 0.08 * atmos(E4) >> swell(3.0, 2.0) for 8 beats
  at 8  play 0.15 * atmos(Bb2) >> swell(2.0, 2.0) for 8 beats
  at 8  play 0.12 * atmos(D3) >> swell(2.0, 2.0) for 8 beats
  at 8  play 0.10 * atmos(F3) >> swell(2.0, 2.0) for 8 beats
  at 8  play 0.08 * atmos(A3) >> swell(2.0, 2.0) for 8 beats
  at 16 play 0.15 * atmos(G2) >> swell(2.0, 2.0) for 8 beats
  at 16 play 0.12 * atmos(Bb2) >> swell(2.0, 2.0) for 8 beats
  at 16 play 0.10 * atmos(D3) >> swell(2.0, 2.0) for 8 beats
  at 16 play 0.08 * atmos(F3) >> swell(2.0, 2.0) for 8 beats
  at 24 play 0.15 * atmos(A2) >> swell(2.0, 2.0) for 8 beats
  at 24 play 0.12 * atmos(C3) >> swell(2.0, 2.0) for 8 beats
  at 24 play 0.10 * atmos(E3) >> swell(2.0, 2.0) for 8 beats
  at 24 play 0.08 * atmos(G3) >> swell(2.0, 2.0) for 8 beats

// Silk pad — lighter, comes in later for lift
pattern silk_wash = 32 beats
  at 0  play 0.1 * silk(D4) >> swell(4.0, 3.0) for 16 beats
  at 0  play 0.08 * silk(A4) >> swell(4.0, 3.0) for 16 beats
  at 16 play 0.1 * silk(Bb3) >> swell(4.0, 3.0) for 16 beats
  at 16 play 0.08 * silk(F4) >> swell(4.0, 3.0) for 16 beats

// Bass — melodic, follows chord roots with movement
pattern bass_a = 32 beats
  at 0  play 0.3 * deep(D2) >> swell(0.1, 0.5) for 3 beats
  at 4  play 0.25 * deep(D2) >> swell(0.05, 0.3) for 2 beats
  at 7  play 0.2 * deep(E2) >> swell(0.05, 0.3) for 1 beat
  at 8  play 0.3 * deep(Bb1) >> swell(0.1, 0.5) for 3 beats
  at 12 play 0.25 * deep(Bb1) >> swell(0.05, 0.3) for 2 beats
  at 15 play 0.2 * deep(C2) >> swell(0.05, 0.3) for 1 beat
  at 16 play 0.3 * deep(G1) >> swell(0.1, 0.5) for 3 beats
  at 20 play 0.25 * deep(G1) >> swell(0.05, 0.3) for 2 beats
  at 23 play 0.2 * deep(A1) >> swell(0.05, 0.3) for 1 beat
  at 24 play 0.3 * deep(A1) >> swell(0.1, 0.5) for 3 beats
  at 28 play 0.25 * deep(A1) >> swell(0.05, 0.3) for 2 beats
  at 31 play 0.2 * deep(D2) >> swell(0.05, 0.3) for 1 beat

// -- Melodic layers --

// Plucked melody — sparse, Dm pentatonic, lots of space
pattern melody_pluck = 32 beats
  at 1   play 0.15 * pluck(A4) >> swell(0.0, 0.3) for 2 beats
  at 5   play 0.12 * pluck(F4) >> swell(0.0, 0.3) for 1.5 beats
  at 8   play 0.15 * pluck(D4) >> swell(0.0, 0.3) for 3 beats
  at 13  play 0.12 * pluck(C5) >> swell(0.0, 0.3) for 1.5 beats
  at 16  play 0.15 * pluck(A4) >> swell(0.0, 0.3) for 2 beats
  at 19  play 0.1 * pluck(G4) >> swell(0.0, 0.3) for 1 beat
  at 21  play 0.12 * pluck(F4) >> swell(0.0, 0.3) for 2 beats
  at 25  play 0.15 * pluck(E4) >> swell(0.0, 0.3) for 2 beats
  at 29  play 0.1 * pluck(D4) >> swell(0.0, 0.3) for 2 beats

// Kalimba response — high register answers to the pluck melody
pattern melody_thumb = 32 beats
  at 3   play 0.1 * thumb(D5) >> swell(0.0, 0.2) for 1 beat
  at 7   play 0.08 * thumb(A5) >> swell(0.0, 0.2) for 1 beat
  at 11  play 0.1 * thumb(F5) >> swell(0.0, 0.2) for 1.5 beats
  at 15  play 0.08 * thumb(E5) >> swell(0.0, 0.2) for 1 beat
  at 18  play 0.1 * thumb(D5) >> swell(0.0, 0.2) for 1 beat
  at 23  play 0.08 * thumb(G5) >> swell(0.0, 0.2) for 1 beat
  at 27  play 0.1 * thumb(A5) >> swell(0.0, 0.2) for 1.5 beats
  at 31  play 0.08 * thumb(F5) >> swell(0.0, 0.2) for 1 beat

// Shimmer accents — very sparse, high glass tones
pattern glass_drops = 32 beats
  at 2   play 0.06 * shimmer(D6) >> swell(0.0, 0.2) for 2 beats
  at 14  play 0.05 * shimmer(A5) >> swell(0.0, 0.2) for 2 beats
  at 22  play 0.06 * shimmer(F5) >> swell(0.0, 0.2) for 2 beats

// -- Texture layers --

pattern texture = 32 beats
  at 0 play earth for 32 beats
  at 0 play air for 32 beats
  at 0 play hum for 32 beats

pattern hum_down = 16 beats
  at 0 play hum >> swell(0.2, 0.5) for 16 beats

// ============================================================
// SECTIONS
// ============================================================

// Intro — just texture and pads, no rhythm
section intro = 32 beats
  play chords_a
  play texture

// Texture + sparse beat entering
section drums_enter = 32 beats
  play chords_a
  play texture
  repeat beat_sparse every 8 beats

// Add bass — the groove locks in
section groove = 32 beats
  play chords_a
  play bass_a
  play texture
  repeat beat_sparse every 8 beats
  repeat shakers every 4 beats

// Full arrangement — melody enters
section full_a = 32 beats
  play chords_a
  play bass_a
  play texture
  play melody_pluck
  repeat beat_full every 8 beats
  repeat shakers every 4 beats
  repeat hats every 8 beats

// With kalimba response — call and answer
section full_b = 32 beats
  play chords_a
  play silk_wash
  play bass_a
  play texture
  play melody_pluck
  play melody_thumb
  repeat beat_full every 8 beats
  repeat shakers every 4 beats
  repeat hats every 8 beats

// Peak — everything layered, glass drops for sparkle
section peak = 32 beats
  play chords_a
  play silk_wash
  play bass_a
  play texture
  play melody_pluck
  play melody_thumb
  play glass_drops
  repeat beat_full every 8 beats
  repeat shakers every 4 beats
  repeat hats every 8 beats

// Wind down — drop the drums, melody thins
section wind_down = 32 beats
  play chords_a
  play bass_a
  play texture
  play melody_pluck
  repeat beat_full from 8

// Outro — pads and texture fading
section outro = 34 beats
  repeat chords_a to 32
  repeat texture until 32
  repeat silk_wash until 32
  repeat melody_thumb to 30
  repeat beat_sparse to 32
  at 24 play hum_down

// ============================================================
// ARRANGEMENT
// ============================================================

// Patient build over ~3 minutes at 98 bpm
play intro
play drums_enter
play groove
play full_a
play full_b
play peak
play peak
play full_a
play wind_down
play outro
