// three-faces.sc — Three genre treatments of a famous romantic piano theme
// Same harmonic skeleton, three wildly different interpretations:
//   1. Jazz lounge (0:00 - 1:20)
//   2. Ragtime stride (1:20 - 2:20)
//   3. Drum & bass (2:20 - 3:20)
//
// Source material: the iconic bell chords and descending bass from
// a well-known piano concerto opening. Fm → Fm/Db → Fm/D → Bbm → C7 → Fm

import voices/instruments.sc
import voices/organic-kit.sc
import voices/drive-kit.sc

// Mix levels — instruments from instruments.sc
normalize rhodes 0.3
normalize tine 0.3
normalize glass 0.3
normalize muted_piano 0.3
normalize vibes 0.3
normalize pluck_bass 0.3
normalize fm_bass 0.3
normalize warm_pad 0.2
// instruments from organic-kit.sc
normalize pluck 0.3
normalize thumb 0.3
normalize shimmer 0.2
normalize deep 0.3
normalize atmos 0.2
normalize silk 0.2
// instruments from drive-kit.sc
normalize supersaw 0.35
normalize syn_pluck 0.3
normalize saw_bass 0.3
normalize massive 0.2
normalize soft_pad 0.2
// voices from organic-kit.sc
normalize dk 0.12
normalize sn 0.18
normalize rim 0.15
normalize ht 0.12
normalize oht 0.15
// voices from drive-kit.sc
normalize kick4 0.12
normalize clap 0.18
normalize chh 0.1
normalize ohh 0.12
normalize ride 0.12
normalize impact 0.15

master compress 1.0
master gain -3

// ============================================================
// SHARED — the harmonic DNA
// ============================================================

// The progression: Fm → Fm(add Db) → Fm(add D) → Bbm → C7 → Fm
// Rach's bell chords, reharmonized for each genre

// ============================================================
// PART 1: JAZZ LOUNGE — 88 bpm, heavy swing, Rhodes + walking bass
// ============================================================

bpm 88
swing 0.65
humanize 10

// Jazz voicings — Rhodes with 9ths and extensions
fx smoky = reverb(0.5, 0.4, 0.3) >> lowpass(3000, 0.5, 0.3)

// Bell chord theme on Rhodes — voiced open, jazz extensions
pattern jazz_bells = 16 beats
  at 0  play 0.15 * rhodes(F3) >> swell(0.0, 0.5) for 4 beats
  at 0  play 0.12 * rhodes(Ab3) >> swell(0.0, 0.5) for 4 beats
  at 0  play 0.10 * rhodes(C4) >> swell(0.0, 0.5) for 4 beats
  at 0  play 0.08 * rhodes(Eb4) >> swell(0.0, 0.5) for 4 beats
  at 4  play 0.15 * rhodes(Db3) >> swell(0.0, 0.5) for 4 beats
  at 4  play 0.12 * rhodes(F3) >> swell(0.0, 0.5) for 4 beats
  at 4  play 0.10 * rhodes(Ab3) >> swell(0.0, 0.5) for 4 beats
  at 4  play 0.08 * rhodes(C4) >> swell(0.0, 0.5) for 4 beats
  at 8  play 0.15 * rhodes(Bb2) >> swell(0.0, 0.5) for 4 beats
  at 8  play 0.12 * rhodes(Db3) >> swell(0.0, 0.5) for 4 beats
  at 8  play 0.10 * rhodes(F3) >> swell(0.0, 0.5) for 4 beats
  at 8  play 0.08 * rhodes(Ab3) >> swell(0.0, 0.5) for 4 beats
  at 12 play 0.15 * rhodes(C3) >> swell(0.0, 0.5) for 4 beats
  at 12 play 0.12 * rhodes(E3) >> swell(0.0, 0.5) for 4 beats
  at 12 play 0.10 * rhodes(G3) >> swell(0.0, 0.5) for 4 beats
  at 12 play 0.08 * rhodes(Bb3) >> swell(0.0, 0.5) for 4 beats

// Walking bass — descending chromatic approach tones
pattern jazz_walk = 16 beats
  at 0   play 0.25 * pluck_bass(F2) >> swell(0.0, 0.2) for 1.5 beats
  at 1.5 play 0.2 * pluck_bass(Ab2) >> swell(0.0, 0.2) for 1 beat
  at 3   play 0.2 * pluck_bass(C2) >> swell(0.0, 0.2) for 1 beat
  at 4   play 0.25 * pluck_bass(Db2) >> swell(0.0, 0.2) for 1.5 beats
  at 5.5 play 0.2 * pluck_bass(F2) >> swell(0.0, 0.2) for 1 beat
  at 7   play 0.2 * pluck_bass(Ab2) >> swell(0.0, 0.2) for 1 beat
  at 8   play 0.25 * pluck_bass(Bb1) >> swell(0.0, 0.2) for 1.5 beats
  at 9.5 play 0.2 * pluck_bass(Db2) >> swell(0.0, 0.2) for 1 beat
  at 11  play 0.2 * pluck_bass(F2) >> swell(0.0, 0.2) for 1 beat
  at 12  play 0.25 * pluck_bass(C2) >> swell(0.0, 0.2) for 1.5 beats
  at 13.5 play 0.2 * pluck_bass(E2) >> swell(0.0, 0.2) for 1 beat
  at 15  play 0.2 * pluck_bass(F2) >> swell(0.0, 0.2) for 1 beat

// Vibes melody — the descending theme, reimagined
pattern jazz_melody = 16 beats
  at 0.5 play 0.12 * vibes(C5) >> swell(0.0, 0.3) for 2 beats
  at 3   play 0.10 * vibes(Ab4) >> swell(0.0, 0.3) for 1.5 beats
  at 5   play 0.12 * vibes(F4) >> swell(0.0, 0.3) for 2 beats
  at 8   play 0.10 * vibes(Db5) >> swell(0.0, 0.3) for 2 beats
  at 11  play 0.12 * vibes(Bb4) >> swell(0.0, 0.3) for 2 beats
  at 14  play 0.10 * vibes(G4) >> swell(0.0, 0.3) for 2 beats

// Jazz brushes — ride + ghost snare, very swung
pattern jazz_drums = 8 beats swing 0.67 humanize 12
  at 0   play dk for 0.3 beats
  at 1   play rim for 0.15 beats
  at 2   play dk for 0.3 beats
  at 2.5 play sn for 0.15 beats
  at 3   play rim for 0.15 beats
  at 4   play dk for 0.3 beats
  at 5.5 play rim for 0.15 beats
  at 6   play dk for 0.3 beats
  at 7   play sn for 0.15 beats
  at 7.5 play rim for 0.15 beats

// Ride pattern
pattern jazz_ride = 4 beats swing 0.67 humanize 8
  at 0   play ht for 0.2 beats
  at 1   play ht for 0.2 beats
  at 1.5 play ht for 0.15 beats
  at 2   play ht for 0.2 beats
  at 3   play ht for 0.2 beats
  at 3.5 play ht for 0.15 beats

// Sections
section jazz_intro = 16 beats
  play jazz_bells

section jazz_full = 16 beats
  play jazz_bells
  play jazz_walk
  play jazz_melody
  repeat jazz_drums every 8 beats
  repeat jazz_ride every 4 beats

// ============================================================
// PART 2: RAGTIME STRIDE — 130 bpm, straight, bouncy
// ============================================================

// Stride piano — alternating bass on 1&3, chord on 2&4
// The theme becomes a syncopated right-hand melody

pattern rag_stride = 16 beats
  // Left hand: bass-chord-bass-chord
  at 0   play 0.2 * tine(F2) >> swell(0.0, 0.15) for 0.5 beats
  at 0.5 play 0.12 * tine(Ab3) >> swell(0.0, 0.1) for 0.5 beats
  at 0.5 play 0.10 * tine(C4) >> swell(0.0, 0.1) for 0.5 beats
  at 1   play 0.2 * tine(C2) >> swell(0.0, 0.15) for 0.5 beats
  at 1.5 play 0.12 * tine(Ab3) >> swell(0.0, 0.1) for 0.5 beats
  at 1.5 play 0.10 * tine(C4) >> swell(0.0, 0.1) for 0.5 beats
  at 2   play 0.2 * tine(F2) >> swell(0.0, 0.15) for 0.5 beats
  at 2.5 play 0.12 * tine(Ab3) >> swell(0.0, 0.1) for 0.5 beats
  at 2.5 play 0.10 * tine(C4) >> swell(0.0, 0.1) for 0.5 beats
  at 3   play 0.2 * tine(C2) >> swell(0.0, 0.15) for 0.5 beats
  at 3.5 play 0.12 * tine(Ab3) >> swell(0.0, 0.1) for 0.5 beats
  at 3.5 play 0.10 * tine(C4) >> swell(0.0, 0.1) for 0.5 beats
  // Db chord
  at 4   play 0.2 * tine(Db2) >> swell(0.0, 0.15) for 0.5 beats
  at 4.5 play 0.12 * tine(F3) >> swell(0.0, 0.1) for 0.5 beats
  at 4.5 play 0.10 * tine(Ab3) >> swell(0.0, 0.1) for 0.5 beats
  at 5   play 0.2 * tine(Ab1) >> swell(0.0, 0.15) for 0.5 beats
  at 5.5 play 0.12 * tine(F3) >> swell(0.0, 0.1) for 0.5 beats
  at 5.5 play 0.10 * tine(Ab3) >> swell(0.0, 0.1) for 0.5 beats
  at 6   play 0.2 * tine(Db2) >> swell(0.0, 0.15) for 0.5 beats
  at 6.5 play 0.12 * tine(F3) >> swell(0.0, 0.1) for 0.5 beats
  at 6.5 play 0.10 * tine(Ab3) >> swell(0.0, 0.1) for 0.5 beats
  at 7   play 0.2 * tine(Ab1) >> swell(0.0, 0.15) for 0.5 beats
  at 7.5 play 0.12 * tine(F3) >> swell(0.0, 0.1) for 0.5 beats
  at 7.5 play 0.10 * tine(Ab3) >> swell(0.0, 0.1) for 0.5 beats
  // Bbm
  at 8   play 0.2 * tine(Bb1) >> swell(0.0, 0.15) for 0.5 beats
  at 8.5 play 0.12 * tine(Db3) >> swell(0.0, 0.1) for 0.5 beats
  at 8.5 play 0.10 * tine(F3) >> swell(0.0, 0.1) for 0.5 beats
  at 9   play 0.2 * tine(F2) >> swell(0.0, 0.15) for 0.5 beats
  at 9.5 play 0.12 * tine(Db3) >> swell(0.0, 0.1) for 0.5 beats
  at 9.5 play 0.10 * tine(F3) >> swell(0.0, 0.1) for 0.5 beats
  at 10  play 0.2 * tine(Bb1) >> swell(0.0, 0.15) for 0.5 beats
  at 10.5 play 0.12 * tine(Db3) >> swell(0.0, 0.1) for 0.5 beats
  at 10.5 play 0.10 * tine(F3) >> swell(0.0, 0.1) for 0.5 beats
  at 11  play 0.2 * tine(F2) >> swell(0.0, 0.15) for 0.5 beats
  at 11.5 play 0.12 * tine(Db3) >> swell(0.0, 0.1) for 0.5 beats
  at 11.5 play 0.10 * tine(F3) >> swell(0.0, 0.1) for 0.5 beats
  // C7 → Fm resolution
  at 12  play 0.2 * tine(C2) >> swell(0.0, 0.15) for 0.5 beats
  at 12.5 play 0.12 * tine(E3) >> swell(0.0, 0.1) for 0.5 beats
  at 12.5 play 0.10 * tine(Bb3) >> swell(0.0, 0.1) for 0.5 beats
  at 13  play 0.2 * tine(G1) >> swell(0.0, 0.15) for 0.5 beats
  at 13.5 play 0.12 * tine(E3) >> swell(0.0, 0.1) for 0.5 beats
  at 13.5 play 0.10 * tine(Bb3) >> swell(0.0, 0.1) for 0.5 beats
  at 14  play 0.2 * tine(C2) >> swell(0.0, 0.15) for 0.5 beats
  at 14.5 play 0.12 * tine(F3) >> swell(0.0, 0.1) for 0.5 beats
  at 14.5 play 0.10 * tine(Ab3) >> swell(0.0, 0.1) for 0.5 beats
  at 15  play 0.2 * tine(F2) >> swell(0.0, 0.15) for 0.5 beats
  at 15.5 play 0.12 * tine(F3) >> swell(0.0, 0.1) for 0.5 beats
  at 15.5 play 0.10 * tine(Ab3) >> swell(0.0, 0.1) for 0.5 beats

// Right hand melody — syncopated, bouncy
pattern rag_melody = 16 beats
  at 0   play 0.15 * glass(C5) >> swell(0.0, 0.15) for 0.5 beats
  at 0.5 play 0.12 * glass(Ab4) >> swell(0.0, 0.15) for 0.5 beats
  at 1.5 play 0.15 * glass(F4) >> swell(0.0, 0.15) for 1 beat
  at 3   play 0.12 * glass(Ab4) >> swell(0.0, 0.15) for 0.5 beats
  at 3.5 play 0.15 * glass(C5) >> swell(0.0, 0.15) for 0.5 beats
  at 4   play 0.12 * glass(Db5) >> swell(0.0, 0.15) for 1 beat
  at 5.5 play 0.15 * glass(C5) >> swell(0.0, 0.15) for 0.5 beats
  at 6   play 0.12 * glass(Ab4) >> swell(0.0, 0.15) for 1 beat
  at 7.5 play 0.15 * glass(F4) >> swell(0.0, 0.15) for 0.5 beats
  at 8   play 0.12 * glass(Db5) >> swell(0.0, 0.15) for 0.5 beats
  at 8.5 play 0.15 * glass(Bb4) >> swell(0.0, 0.15) for 1 beat
  at 10  play 0.12 * glass(Ab4) >> swell(0.0, 0.15) for 0.5 beats
  at 10.5 play 0.15 * glass(F4) >> swell(0.0, 0.15) for 0.5 beats
  at 12  play 0.15 * glass(E4) >> swell(0.0, 0.15) for 0.5 beats
  at 12.5 play 0.12 * glass(G4) >> swell(0.0, 0.15) for 0.5 beats
  at 13  play 0.15 * glass(Bb4) >> swell(0.0, 0.15) for 1 beat
  at 14.5 play 0.12 * glass(Ab4) >> swell(0.0, 0.15) for 0.5 beats
  at 15  play 0.15 * glass(F4) >> swell(0.0, 0.15) for 1 beat

section rag = 16 beats
  play rag_stride
  play rag_melody

// ============================================================
// PART 3: DRUM & BASS — 170 bpm, breakbeats, degraded melody
// ============================================================

// The theme is chopped, crushed, and played in half-time
// over a relentless breakbeat

fx shred = crush(8) >> compress(-15, 4, 0.003, 0.05) >> lowpass(3000, 0.5)

// Chopped melody — half-time fragments of the theme, degraded
pattern dnb_chops = 16 beats
  at 0   play 0.2 * muted_piano(C5) >> shred >> swell(0.0, 0.3) for 2 beats
  at 4   play 0.18 * muted_piano(Ab4) >> shred >> swell(0.0, 0.3) for 2 beats
  at 8   play 0.2 * muted_piano(F4) >> shred >> swell(0.0, 0.5) for 3 beats
  at 12  play 0.18 * muted_piano(Db5) >> shred >> swell(0.0, 0.3) for 2 beats
  at 14  play 0.15 * muted_piano(C5) >> shred >> swell(0.0, 0.3) for 2 beats

// Reese bass — deep, distorted, half-time
pattern dnb_bass = 16 beats
  at 0  play 0.4 * fm_bass(F2) >> swell(0.1, 0.5) for 4 beats
  at 4  play 0.35 * fm_bass(Db2) >> swell(0.1, 0.5) for 4 beats
  at 8  play 0.4 * fm_bass(Bb1) >> swell(0.1, 0.5) for 4 beats
  at 12 play 0.35 * fm_bass(C2) >> swell(0.1, 0.5) for 4 beats

// Breakbeat — fast, aggressive
pattern dnb_break = 4 beats
  at 0    play kick4 for 0.2 beats
  at 0.5  play chh for 0.1 beats
  at 1    play clap for 0.2 beats
  at 1.5  play chh for 0.1 beats
  at 2    play kick4 for 0.2 beats
  at 2.25 play kick4 for 0.2 beats
  at 2.5  play chh for 0.1 beats
  at 3    play clap for 0.2 beats
  at 3.25 play chh for 0.1 beats
  at 3.5  play kick4 for 0.2 beats
  at 3.75 play chh for 0.1 beats

// Ghost hats for texture
pattern dnb_hats = 4 beats humanize 5
  at 0   play chh for 0.1 beats
  at 0.5 play chh for 0.1 beats
  at 1   play ohh for 0.15 beats
  at 1.5 play chh for 0.1 beats
  at 2   play chh for 0.1 beats
  at 2.5 play chh for 0.1 beats
  at 3   play ohh for 0.15 beats
  at 3.5 play chh for 0.1 beats

// Pad stab — atmospheric, degraded
pattern dnb_pad = 16 beats
  at 0  play 0.08 * warm_pad(F3) >> crush(10) >> swell(1.0, 1.0) for 8 beats
  at 0  play 0.06 * warm_pad(Ab3) >> crush(10) >> swell(1.0, 1.0) for 8 beats
  at 0  play 0.05 * warm_pad(C4) >> crush(10) >> swell(1.0, 1.0) for 8 beats
  at 8  play 0.08 * warm_pad(Bb2) >> crush(10) >> swell(1.0, 1.0) for 8 beats
  at 8  play 0.06 * warm_pad(Db3) >> crush(10) >> swell(1.0, 1.0) for 8 beats
  at 8  play 0.05 * warm_pad(F3) >> crush(10) >> swell(1.0, 1.0) for 8 beats

section dnb_intro = 16 beats
  play dnb_chops
  play dnb_pad

section dnb_full = 16 beats
  play dnb_chops
  play dnb_bass
  play dnb_pad
  repeat dnb_break every 4 beats
  repeat dnb_hats every 4 beats

// ============================================================
// ARRANGEMENT — Three faces of one theme
// ============================================================

// Part 1: Jazz lounge
play jazz_intro
play jazz_full
play jazz_full
play jazz_full

// Part 2: Ragtime — tempo change!
bpm 130

play rag
play rag
play rag

// Part 3: DnB — tempo change!
bpm 170

play dnb_intro
play dnb_full
play dnb_full
play dnb_full
