// therapy-lofi.sc — Therapeutic lo-fi beat
// ~4 minutes at 68 BPM
//
// Layers 40 Hz gamma entrainment (click-train approach),
// alpha-range shimmer, and sub-bass in the bone-resonant range
// under a warm lo-fi beat. Gamma voices alternate across sections
// and fade in/out like insect waves using swell().

import voices/therapy-kit.sc

bpm 68

// --- Patterns ---

// 4-bar chord cycle: Dm9 → Bbmaj7 → Gm7 → A7
pattern chords_cycle = 16 beats
  at 0 play chord_pad(Dm9) for 4 beats
  at 4 play chord_pad(Bbmaj7) for 4 beats
  at 8 play chord_pad(Gm7) for 4 beats
  at 12 play chord_pad(Adom7) for 4 beats

// Bass follows root
pattern bass_cycle = 16 beats
  at 0 play bass(D1) for 3.5 beats
  at 4 play bass(Bb0) for 3.5 beats
  at 8 play bass(G1) for 3.5 beats
  at 12 play bass(A1) for 3.5 beats

// Basic boom-bap
pattern drums_basic = 4 beats
  at 0 play kick for 0.5 beats
  at 1 play snare for 0.5 beats
  at 2 play kick for 0.5 beats
  at 3 play snare for 0.5 beats

// With ghost notes
pattern drums_ghost = 4 beats
  at 0 play kick for 0.5 beats
  at 0.75 play ghost for 0.3 beats
  at 1 play snare for 0.5 beats
  at 2 play kick for 0.5 beats
  at 2.5 play ghost for 0.3 beats
  at 3 play snare for 0.5 beats

// Hats
pattern hats_sparse = 4 beats
  at 0.5 play hat for 0.2 beats
  at 2.5 play hat for 0.2 beats

pattern hats_busy = 4 beats
  at 0.5 play hat for 0.2 beats
  at 1.5 play hat for 0.2 beats
  at 2.5 play hat for 0.2 beats
  at 3.5 play hat for 0.15 beats

// Vinyl crackle — RPM-accurate, clustered imperfections
pattern vinyl_rotation = 2 beats
  at 0.0 play pop for 0.1 beats
  at 0.15 play click for 0.05 beats
  at 1.1 play click for 0.05 beats

// Melody — flowing lines with swell for soft attack
// Full D natural minor: D E F G A Bb C — Billy Joel piano feel

// Melody A: rising runs, singable phrases — eighth-note grid
pattern melody_a = 16 beats
  // Over Dm9: rising run
  at 0.5 play mel_low(D4) >> swell(0.05, 0.2) for 0.5 beats
  at 1.0 play mel_low(F4) >> swell(0.05, 0.2) for 0.5 beats
  at 1.5 play mel(A4) >> swell(0.05, 0.3) for 0.5 beats
  at 2.0 play mel(D5) >> swell(0.05, 0.5) for 1.5 beats
  // Over Bbmaj7: descending answer
  at 4.5 play mel(D5) >> swell(0.05, 0.3) for 0.5 beats
  at 5.0 play mel(Bb4) >> swell(0.05, 0.4) for 0.5 beats
  at 5.5 play mel(A4) >> swell(0.05, 0.3) for 0.5 beats
  at 6.0 play mel_low(F4) >> swell(0.05, 0.5) for 1.5 beats
  // Over Gm7: rising phrase
  at 8.0 play mel_low(G4) >> swell(0.05, 0.2) for 0.5 beats
  at 8.5 play mel(Bb4) >> swell(0.05, 0.2) for 0.5 beats
  at 9.0 play mel(C5) >> swell(0.05, 0.3) for 0.5 beats
  at 9.5 play mel(D5) >> swell(0.05, 0.5) for 1.5 beats
  // Over A7: resolve
  at 13.0 play mel(E5) >> swell(0.08, 0.4) for 0.5 beats
  at 13.5 play mel(C5) >> swell(0.05, 0.6) for 2.0 beats

// Melody B: descending, spacious, lyrical
pattern melody_b = 16 beats
  // Over Dm9: high start, drift down
  at 0.5 play mel(F5) >> swell(0.08, 0.4) for 1.0 beats
  at 1.5 play mel(D5) >> swell(0.05, 0.5) for 1.0 beats
  at 3.0 play mel(A4) >> swell(0.05, 0.4) for 1.0 beats
  // Over Bbmaj7: gentle rising turn
  at 5.0 play mel(Bb4) >> swell(0.05, 0.3) for 0.5 beats
  at 5.5 play mel(C5) >> swell(0.05, 0.5) for 1.0 beats
  at 7.0 play mel(D5) >> swell(0.08, 0.4) for 1.0 beats
  // Over Gm7: spacious arc
  at 9.0 play mel(G5) >> swell(0.08, 0.5) for 1.5 beats
  at 10.5 play mel(D5) >> swell(0.05, 0.4) for 1.0 beats
  // Over A7: low resolve
  at 12.5 play mel(C5) >> swell(0.05, 0.3) for 0.5 beats
  at 13.0 play mel(A4) >> swell(0.05, 0.5) for 1.0 beats
  at 14.0 play mel_low(E4) >> swell(0.05, 0.5) for 1.5 beats

// Melody C: rhythmic repeating-figure feel
pattern melody_c = 16 beats
  // Over Dm9: call
  at 0.0 play mel(A4) >> swell(0.05, 0.2) for 0.5 beats
  at 0.5 play mel(D5) >> swell(0.05, 0.2) for 0.5 beats
  at 1.5 play mel(F5) >> swell(0.05, 0.5) for 1.5 beats
  at 3.5 play mel(D5) >> swell(0.05, 0.3) for 0.5 beats
  // Over Bbmaj7: answer
  at 4.5 play mel(Bb4) >> swell(0.05, 0.3) for 0.5 beats
  at 5.0 play mel(C5) >> swell(0.05, 0.3) for 0.5 beats
  at 5.5 play mel(D5) >> swell(0.05, 0.5) for 2.0 beats
  // Over Gm7: building
  at 8.0 play mel_low(G4) >> swell(0.05, 0.2) for 0.5 beats
  at 8.5 play mel(Bb4) >> swell(0.05, 0.2) for 0.5 beats
  at 9.0 play mel(D5) >> swell(0.05, 0.3) for 0.5 beats
  at 9.5 play mel(F5) >> swell(0.05, 0.5) for 1.5 beats
  // Over A7: resolution
  at 12.5 play mel(C5) >> swell(0.05, 0.5) for 1.0 beats
  at 13.5 play mel(A4) >> swell(0.05, 0.6) for 2.0 beats

// Light melody — sparse hints, for early sections before full melody enters
pattern melody_light = 16 beats
  at 2.0 play mel(D5) >> swell(0.08, 0.5) for 1.5 beats
  at 5.5 play mel(F5) >> swell(0.08, 0.5) for 1.2 beats
  at 10.0 play mel(D5) >> swell(0.08, 0.6) for 1.8 beats

// --- Therapeutic tone combos (gamma fades in/out like insect waves) ---

// Crisp cicada clicks with swell + alpha + bone drone
pattern therapy_crisp = 16 beats
  at 0 play gamma_clicks >> swell(2.0, 2.0) for 16 beats
  at 0 play alpha_shimmer for 16 beats
  at 0 play bone_drone for 16 beats

// Soft wash with swell + alpha + bone drone
pattern therapy_wash = 16 beats
  at 0 play gamma_soft >> swell(2.5, 2.5) for 16 beats
  at 0 play alpha_shimmer for 16 beats
  at 0 play bone_drone for 16 beats

// Both gamma voices combined (climax) — staggered swells
pattern therapy_full = 16 beats
  at 0 play gamma_clicks >> swell(1.5, 2.0) for 16 beats
  at 0 play gamma_soft >> swell(2.5, 1.5) for 16 beats
  at 0 play alpha_shimmer for 16 beats
  at 0 play bone_drone for 16 beats

// Vinyl + hiss texture
pattern texture = 16 beats
  at 0 play hiss for 16 beats

// Intro/outro patterns
pattern intro_bed = 4 beats
  at 0 play gamma_clicks >> swell(1.5, 1.0) for 4 beats
  at 0 play hiss for 4 beats

pattern outro_bed = 8 beats
  at 0 play gamma_soft >> swell(0.5, 4.0) for 8 beats
  at 0 play hiss for 8 beats
  at 1.5 play pop for 0.1 beats

// --- Sections ---

// Short intro: gamma fades in over vinyl
section intro = 4 beats
  play intro_bed

// Basic verse: soft gamma, no melody
section verse_soft = 16 beats
  play therapy_wash
  play texture
  play chords_cycle
  play bass_cycle
  repeat drums_basic every 4 beats
  repeat hats_sparse every 4 beats
  repeat vinyl_rotation every 2 beats

// Verse with light melody hints — melody starts emerging
section verse_soft_mel = 16 beats
  play therapy_wash
  play texture
  play chords_cycle
  play bass_cycle
  play melody_light
  repeat drums_basic every 4 beats
  repeat hats_sparse every 4 beats
  repeat vinyl_rotation every 2 beats

// Crisp gamma with light melody
section verse_crisp_mel = 16 beats
  play therapy_crisp
  play texture
  play chords_cycle
  play bass_cycle
  play melody_light
  repeat drums_basic every 4 beats
  repeat hats_sparse every 4 beats
  repeat vinyl_rotation every 2 beats

// Full verse: crisp gamma + ghost drums + melody A
section verse_full_crisp = 16 beats
  play therapy_crisp
  play texture
  play chords_cycle
  play bass_cycle
  play melody_a
  repeat drums_ghost every 4 beats
  repeat hats_busy every 4 beats
  repeat vinyl_rotation every 2 beats

// Full verse: soft gamma + ghost drums + melody B
section verse_full_wash = 16 beats
  play therapy_wash
  play texture
  play chords_cycle
  play bass_cycle
  play melody_b
  repeat drums_ghost every 4 beats
  repeat hats_busy every 4 beats
  repeat vinyl_rotation every 2 beats

// Full verse: crisp gamma + ghost drums + melody C (rhythmic)
section verse_full_c = 16 beats
  play therapy_crisp
  play texture
  play chords_cycle
  play bass_cycle
  play melody_c
  repeat drums_ghost every 4 beats
  repeat hats_busy every 4 beats
  repeat vinyl_rotation every 2 beats

// Climax: both gamma + full arrangement + melody A
section climax = 16 beats
  play therapy_full
  play texture
  play chords_cycle
  play bass_cycle
  play melody_a
  repeat drums_ghost every 4 beats
  repeat hats_busy every 4 beats
  repeat vinyl_rotation every 2 beats

// Breakdown: drop drums and melody, keep therapy + chords
section breakdown = 16 beats
  play therapy_wash
  play texture
  play chords_cycle
  play bass_cycle
  repeat vinyl_rotation every 2 beats

// Outro: long fade
section outro = 8 beats
  play outro_bed

// --- Arrangement ---
// 4 + 16*17 + 8 = 284 beats
// @ 68 BPM = ~4:11

play intro
play verse_soft
play verse_soft_mel
play verse_crisp_mel
play verse_full_crisp
play verse_full_wash
play breakdown
play climax
repeat 3 {
  pick [verse_full_crisp:2, verse_full_wash:2, verse_full_c:1]
}
play breakdown
play climax
repeat 2 {
  pick [verse_full_wash:2, verse_full_c:2, verse_full_crisp:1]
}
play verse_soft_mel
play verse_soft
play outro
