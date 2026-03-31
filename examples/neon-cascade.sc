// neon-cascade.sc — Progressive electronic
// Driving four-on-the-floor. Long builds. Massive drops.
// ~3:30. Key of F minor.

import voices/drive-kit.sc

// Mix levels
normalize supersaw 0.35
normalize syn_pluck 0.3
normalize saw_bass 0.3
normalize massive 0.2
normalize soft_pad 0.2
normalize kick4 0.12
normalize clap 0.18
normalize chh 0.1
normalize ohh 0.12
normalize ride 0.12
normalize impact 0.15

master compress 1.0
master gain -3

bpm 128

// ============================================================
// PATTERNS
// ============================================================

// -- Kick patterns --

// Four on the floor — the heartbeat
pattern four_floor = 4 beats
  at 0 play kick4 for 0.4 beats
  at 1 play kick4 for 0.4 beats
  at 2 play kick4 for 0.4 beats
  at 3 play kick4 for 0.4 beats

// Half-time kick for breakdowns
pattern kick_half = 4 beats
  at 0 play kick4 for 0.4 beats
  at 2 play kick4 for 0.4 beats

// -- Hat patterns --

pattern hats_driving = 4 beats
  at 0.5 play chh for 0.15 beats
  at 1   play chh for 0.15 beats
  at 1.5 play chh for 0.15 beats
  at 2.5 play chh for 0.15 beats
  at 3   play ohh for 0.25 beats
  at 3.5 play chh for 0.15 beats

pattern hats_buildup = 4 beats
  at 0   play chh for 0.15 beats
  at 0.5 play chh for 0.15 beats
  at 1   play chh for 0.15 beats
  at 1.5 play chh for 0.15 beats
  at 2   play chh for 0.15 beats
  at 2.5 play chh for 0.15 beats
  at 3   play chh for 0.15 beats
  at 3.5 play chh for 0.15 beats

// Ride for energy
pattern rides = 4 beats
  at 0 play ride for 0.3 beats
  at 1 play ride for 0.3 beats
  at 2 play ride for 0.3 beats
  at 3 play ride for 0.3 beats

// Clap on 2 and 4
pattern claps = 4 beats
  at 1 play clap for 0.3 beats
  at 3 play clap for 0.3 beats

// -- Bass patterns --
// Short swell to fake sidechain ducking — bass pumps with the kick

// Bass — sustained notes, one per chord change. The reverb tail on the instrument
// blends note transitions. Swell gives gentle breathing without the tuba effect.
pattern bass_pump = 16 beats
  at 0  play 0.5 * saw_bass(F2) >> swell(0.2, 0.5) for 4.5 beats
  at 4  play 0.5 * saw_bass(Db2) >> swell(0.2, 0.5) for 4.5 beats
  at 8  play 0.5 * saw_bass(Eb2) >> swell(0.2, 0.5) for 4.5 beats
  at 12 play 0.5 * saw_bass(Bb1) >> swell(0.2, 0.5) for 4.5 beats

// -- Chord pads --
// Fm → Db → Eb → Bbm

pattern pads_massive = 16 beats
  at 0  play 0.1 * massive(F3) >> swell(1.0, 0.5) for 4 beats
  at 0  play 0.08 * massive(Ab3) >> swell(1.0, 0.5) for 4 beats
  at 0  play 0.06 * massive(C4) >> swell(1.0, 0.5) for 4 beats
  at 4  play 0.1 * massive(Db3) >> swell(0.5, 0.5) for 4 beats
  at 4  play 0.08 * massive(F3) >> swell(0.5, 0.5) for 4 beats
  at 4  play 0.06 * massive(Ab3) >> swell(0.5, 0.5) for 4 beats
  at 8  play 0.1 * massive(Eb3) >> swell(0.5, 0.5) for 4 beats
  at 8  play 0.08 * massive(G3) >> swell(0.5, 0.5) for 4 beats
  at 8  play 0.06 * massive(Bb3) >> swell(0.5, 0.5) for 4 beats
  at 12 play 0.1 * massive(Bb2) >> swell(0.5, 0.5) for 4 beats
  at 12 play 0.08 * massive(Db3) >> swell(0.5, 0.5) for 4 beats
  at 12 play 0.06 * massive(F3) >> swell(0.5, 0.5) for 4 beats

// Soft pad for breakdowns — louder for presence
pattern pads_soft = 16 beats
  at 0  play 0.2 * soft_pad(F3) >> swell(2.0, 1.0) for 8 beats
  at 0  play 0.16 * soft_pad(Ab3) >> swell(2.0, 1.0) for 8 beats
  at 0  play 0.12 * soft_pad(C4) >> swell(2.0, 1.0) for 8 beats
  at 8  play 0.2 * soft_pad(Eb3) >> swell(2.0, 1.0) for 8 beats
  at 8  play 0.16 * soft_pad(G3) >> swell(2.0, 1.0) for 8 beats
  at 8  play 0.12 * soft_pad(Bb3) >> swell(2.0, 1.0) for 8 beats

// -- Lead melodies --

// Main supersaw lead — big, hooky, Fm pentatonic
// Notes overlap slightly — the reverb tail makes them dissolve into each other
pattern lead_a = 16 beats
  at 0   play 0.22 * supersaw(F4) >> swell(0.1, 0.8) for 3 beats
  at 2   play 0.18 * supersaw(Ab4) >> swell(0.05, 0.8) for 3 beats
  at 4   play 0.22 * supersaw(Bb4) >> swell(0.05, 0.8) for 4 beats
  at 7   play 0.18 * supersaw(Ab4) >> swell(0.05, 0.5) for 2 beats
  at 8   play 0.25 * supersaw(C5) >> swell(0.1, 1.0) for 5 beats
  at 12  play 0.18 * supersaw(Bb4) >> swell(0.05, 0.8) for 3 beats
  at 14  play 0.18 * supersaw(Ab4) >> swell(0.05, 0.8) for 3 beats

// Pluck riff — rhythmic, fills between lead notes
pattern pluck_riff = 16 beats
  at 0.5 play 0.14 * syn_pluck(C5) >> swell(0.0, 0.15) for 0.5 beats
  at 2.5 play 0.12 * syn_pluck(Ab4) >> swell(0.0, 0.15) for 0.5 beats
  at 4.5 play 0.14 * syn_pluck(Bb4) >> swell(0.0, 0.15) for 0.5 beats
  at 6   play 0.12 * syn_pluck(F4) >> swell(0.0, 0.15) for 0.5 beats
  at 8.5 play 0.14 * syn_pluck(Eb5) >> swell(0.0, 0.15) for 0.5 beats
  at 10  play 0.12 * syn_pluck(C5) >> swell(0.0, 0.15) for 0.5 beats
  at 12.5 play 0.14 * syn_pluck(Bb4) >> swell(0.0, 0.15) for 0.5 beats
  at 14.5 play 0.12 * syn_pluck(Ab4) >> swell(0.0, 0.15) for 0.5 beats

// -- Risers and impacts --

pattern riser_16 = 16 beats
  at 0 play riser >> swell(0.0, 1.0) for 16 beats

pattern drop_hit = 4 beats
  at 0 play impact for 4 beats

// ============================================================
// SECTIONS
// ============================================================

// Intro — soft pads, half-time kick, building atmosphere
section intro = 32 beats
  repeat pads_soft every 16 beats
  repeat kick_half every 4 beats

// Build — add hats, accelerating energy
section build_a = 32 beats
  repeat pads_soft every 16 beats
  repeat four_floor every 4 beats
  repeat hats_driving every 4 beats

// Riser into drop
section riser_section = 16 beats
  repeat pads_soft every 16 beats
  repeat hats_buildup every 4 beats
  play riser_16

// THE DROP — everything hits
section drop = 32 beats
  play drop_hit
  repeat pads_massive every 16 beats
  repeat bass_pump every 16 beats
  repeat four_floor every 4 beats
  repeat claps every 4 beats
  repeat hats_driving every 4 beats
  repeat rides every 4 beats
  repeat lead_a every 16 beats

// Drop with pluck fills
section drop_full = 32 beats
  repeat pads_massive every 16 beats
  repeat bass_pump every 16 beats
  repeat four_floor every 4 beats
  repeat claps every 4 beats
  repeat hats_driving every 4 beats
  repeat rides every 4 beats
  repeat lead_a every 16 beats
  repeat pluck_riff every 16 beats

// Breakdown — strip it back, just pads and kick
section breakdown = 32 beats
  repeat pads_soft every 16 beats
  repeat kick_half every 4 beats
  repeat lead_a every 16 beats

// Second riser
section riser_2 = 16 beats
  repeat pads_soft every 16 beats
  repeat hats_buildup every 4 beats
  play riser_16
  repeat lead_a every 16 beats

// Outro — pads fading
section outro = 32 beats
  repeat pads_soft every 16 beats
  repeat kick_half every 4 beats

// ============================================================
// ARRANGEMENT — ~3:30 at 128 BPM
// ============================================================

play intro
play build_a
play riser_section
play drop
play drop_full
play breakdown
play riser_2
play drop_full
play drop_full
play outro
