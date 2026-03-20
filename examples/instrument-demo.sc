// instrument-demo.sc — Showcase of the default instrument library
// Cycles through different instruments so you can hear their character.

import voices/instruments.sc

bpm 90
humanize 5

// -- Patterns for each instrument family --

// Keys showcase
pattern keys_demo = 16 beats
  at 0   play 0.3 * rhodes(C4) >> swell(0.05, 0.5) for 2 beats
  at 2   play 0.3 * rhodes(Eb4) >> swell(0.05, 0.5) for 2 beats
  at 4   play 0.3 * lofi_keys(C4) >> swell(0.05, 0.5) for 2 beats
  at 6   play 0.3 * lofi_keys(Eb4) >> swell(0.05, 0.5) for 2 beats
  at 8   play 0.3 * tine(C5) >> swell(0.05, 0.3) for 2 beats
  at 10  play 0.3 * tine(Eb5) >> swell(0.05, 0.3) for 2 beats
  at 12  play 0.3 * glass(C5) >> swell(0.05, 0.3) for 2 beats
  at 14  play 0.3 * glass(G5) >> swell(0.05, 0.3) for 2 beats

// Plucked strings
pattern pluck_demo = 16 beats
  at 0   play 0.3 * nylon(G3) >> swell(0.0, 0.3) for 2 beats
  at 2   play 0.3 * nylon(C4) >> swell(0.0, 0.3) for 2 beats
  at 4   play 0.3 * steel(G3) >> swell(0.0, 0.3) for 2 beats
  at 6   play 0.3 * steel(C4) >> swell(0.0, 0.3) for 2 beats
  at 8   play 0.3 * harp(C4) >> swell(0.0, 0.3) for 4 beats
  at 12  play 0.3 * pizz(G4) >> swell(0.0, 0.2) for 1 beats
  at 13  play 0.3 * pizz(C5) >> swell(0.0, 0.2) for 1 beats
  at 14  play 0.3 * pizz(Eb5) >> swell(0.0, 0.2) for 1 beats
  at 15  play 0.3 * pizz(G5) >> swell(0.0, 0.2) for 1 beats

// Mallets
pattern mallet_demo = 16 beats
  at 0   play 0.3 * marimba(C4) >> swell(0.0, 0.2) for 2 beats
  at 2   play 0.3 * marimba(G4) >> swell(0.0, 0.2) for 2 beats
  at 4   play 0.3 * vibes(C4) >> swell(0.0, 0.5) for 4 beats
  at 8   play 0.3 * kalimba(C5) >> swell(0.0, 0.3) for 2 beats
  at 10  play 0.3 * kalimba(Eb5) >> swell(0.0, 0.3) for 2 beats
  at 12  play 0.3 * music_box(G5) >> swell(0.0, 0.3) for 2 beats
  at 14  play 0.3 * glock(C6) >> swell(0.0, 0.2) for 2 beats

// Bass showcase
pattern bass_demo = 16 beats
  at 0   play 0.4 * sub(C2) >> swell(0.05, 0.3) for 4 beats
  at 4   play 0.4 * analog_bass(C2) >> swell(0.05, 0.3) for 4 beats
  at 8   play 0.4 * pluck_bass(C2) >> swell(0.0, 0.3) for 4 beats
  at 12  play 0.4 * fm_bass(C2) >> swell(0.0, 0.3) for 4 beats

// Pads — longer notes with texture
pattern pad_demo = 16 beats
  at 0  play 0.2 * warm_pad(C3) >> swell(1.0, 1.0) for 8 beats
  at 0  play 0.2 * warm_pad(Eb3) >> swell(1.0, 1.0) for 8 beats
  at 0  play 0.2 * warm_pad(G3) >> swell(1.0, 1.0) for 8 beats
  at 8  play 0.15 * strings(C3) >> swell(1.0, 1.0) for 8 beats
  at 8  play 0.15 * strings(Eb3) >> swell(1.0, 1.0) for 8 beats
  at 8  play 0.15 * strings(G3) >> swell(1.0, 1.0) for 8 beats

// -- Arrangement --

section demo = 80 beats
  play keys_demo
  play pluck_demo
  play mallet_demo
  play bass_demo
  play pad_demo

play demo
