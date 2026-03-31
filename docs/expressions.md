[← Back to README](../README.md)

# Expressions & Effects Reference

Sound Cabinet expressions describe signal graphs -- chains of oscillators, filters, envelopes, and effects connected by operators. Signal flows left to right through the chain.

## Oscillators

Generate a waveform at a given frequency:

| Function | Sound |
|---|---|
| `sine(A4)` | Pure sine wave -- clean, simple |
| `saw(C3)` | Sawtooth -- bright, buzzy |
| `triangle(E4)` | Triangle -- softer than saw |
| `square(G2)` | Square -- hollow, woody |
| `pulse(C3, 0.25)` | Pulse wave -- variable width (0.0-1.0). 0.5 = square, 0.1 = thin/nasal, 0.9 = bright |
| `noise()` / `white()` | White noise -- flat spectrum, bright. Hi-hats, snares, breath textures |
| `pink()` | Pink noise -- 3 dB/octave rolloff, warmer. Vinyl crackle, ambient, rain |
| `brown()` | Brown noise -- 6 dB/octave rolloff, deep rumble. Thunder, ocean, room tone |

Pulse width can be swept over time using parameter automation:

```sc
pulse(C3, 0.1 -> 0.9)    // classic PWM synth pad sound
```

### Custom Waveforms

Define arbitrary waveform shapes as arrays of sample points. The array represents one cycle -- the oscillator reads through it at the right speed for the frequency, interpolating linearly between points:

```sc
wave plateau = [0.0, 0.4, 0.8, 1.0, 1.0, 1.0, 0.8, 0.4, 0.0, -0.4, -0.8, -1.0, -1.0, -1.0, -0.8, -0.4]
wave spike = [0.0, 1.0, 0.3, 0.1, 0.0, -0.1, -0.3, -1.0]

at 0 play 0.3 * plateau(C3) >> lowpass(2000, 0.7) for 4 beats
at 4 play 0.3 * spike(A4) >> reverb(0.6, 0.4, 0.25) for 4 beats
```

Array length determines resolution:

- Fewer points = crunchier, more aliased (8-bit character)
- More points = smoother, higher fidelity

Waves don't need to be symmetric -- asymmetry adds even harmonics (tube/tape warmth). Custom waves work with instruments, effects, arp, and everything in the pipe chain.

## Filters

Process an incoming signal. Chain after a source with `>>`:

| Function | Effect |
|---|---|
| `lowpass(freq, q)` | Cuts frequencies above `freq`. `q` controls resonance (0.5 = gentle, 2.0 = sharp peak) |
| `lowpass(freq, q, mix)` | Same, with dry/wet blend. `mix` 0.0 = all dry, 1.0 = fully filtered |
| `highpass(freq, q)` | Cuts frequencies below `freq`. Same `q` behavior |
| `highpass(freq, q, mix)` | Same, with dry/wet blend |

### Parameter Automation

Filter frequencies (and pulse width) can sweep over time using the `->` operator:

```sc
saw(C3) >> lowpass(800 -> 4000, 0.7)     // filter opens over event duration
saw(C3) >> highpass(200 -> 50, 0.5)      // highpass sweeps down
pulse(C3, 0.1 -> 0.9)                    // pulse width modulation
```

The sweep is linear from start to end over the duration of the enclosing `for N beats` event.

## Envelopes

Shape amplitude over time:

| Function | Effect |
|---|---|
| `decay(rate)` | Exponential decay: amplitude drops as `e^(-rate * t)`. Higher rate = faster decay |
| `swell(attack, release)` | Fade in over `attack` seconds, fade out over `release` seconds at the end of the event |

### Decay Rate Table

| Sound | Decay | Character |
|---|---|---|
| `decay(8)` | Slow pad release | ~0.3s to near-silence |
| `decay(12)` | Kick drum thump | ~0.25s |
| `decay(15)` | Snare snap | ~0.2s |
| `decay(25)` | Hi-hat "tss" | ~0.12s |
| `decay(40)` | Sharp click | ~0.07s |

## Effects

Effects process a signal in the pipe chain -- place them after the source and any filters:

| Function | Effect | Example |
|---|---|---|
| `lfo(rate, depth)` | Tremolo -- amplitude modulation. `rate` in Hz, `depth` 0.0-1.0 | `saw(C3) >> lfo(6.0, 0.4)` |
| `distort(amount)` | Soft clipping (tanh saturation). 1.0 = subtle warmth, 4.0+ = heavy drive | `saw(C2) >> lowpass(400, 1.2) >> distort(4.0)` |
| `vibrato(rate, depth)` | Pitch wobble via modulated delay. `rate` in Hz, `depth` in samples | `saw(E4) >> vibrato(4.0, 15.0)` |
| `chorus(sep, var, freq)` | Detuned copies for width. `sep`/`var` in seconds, `freq` in Hz | `triangle(E5) >> chorus(0.015, 0.005, 0.3)` |
| `delay(time, fb, mix)` | Feedback delay. `time` in seconds, `fb` 0.0-1.0 (recirculation), `mix` 0.0-1.0 (dry/wet). Auto-damped HF in feedback path | `triangle(G5) >> delay(0.3, 0.5, 0.4)` |
| `reverb(size, damp, mix)` | Freeverb algorithmic reverb. `size` 0.0-1.0 (room size), `damp` 0.0-1.0 (HF absorption), `mix` 0.0-1.0 (dry/wet) | `saw(C4) >> reverb(0.8, 0.4, 0.3)` |
| `compress(thresh, ratio, atk, rel)` | Dynamic range compression. `thresh` in dB, `ratio` e.g. 4 = 4:1, `atk`/`rel` in seconds | `saw(C2) >> compress(-15, 4, 0.01, 0.1)` |
| `crush(bits)` | Bit depth reduction. 8 = retro, 10 = subtle grit, 4 = destroyed | `saw(C3) >> crush(8)` |
| `decimate(factor)` | Sample rate reduction. 2 = half rate, 8 = heavy digital dirt | `sine(A4) >> decimate(4)` |
| `degrade(amount)` | Combined tape/medium degradation (lowpass + decimate + crush + noise). 0.3 = warm, 0.6 = worn tape, 1.0 = destroyed | `triangle(C4) >> degrade(0.5)` |
| `loudness(freq)` | ISO 226 equal-loudness compensation. Frequency-dependent gain so all pitches sound equally loud. Reference: 1 kHz = 0 dB | `saw(freq) >> loudness(freq)` |
| `eq(freq, gain, q)` | Parametric EQ -- peak (bell) filter. Boost or cut `gain` dB at `freq` Hz with bandwidth `q` | `saw(C3) >> eq(400, -3, 1.5)` |
| `eq(freq, gain, low)` | Low shelf -- boost or cut everything below `freq` | `saw(C3) >> eq(80, 4, low)` |
| `eq(freq, gain, high)` | High shelf -- boost or cut everything above `freq` | `sine(A4) >> eq(10000, 2, high)` |
| `bus(name)` | Tag this event's output for sidechain detection | `kick >> bus(drums)` |
| `sidechain(bus, thresh, ratio, atk, rel)` | Duck signal based on a bus level. Classic pumping effect | `pad >> sidechain(drums, -20, 4, 0.01, 0.1)` |

`loudness(freq)` is most useful inside instrument definitions where `freq` is automatically substituted with the note's Hz value. A C2 (65 Hz) gets ~+8 dB, a C4 (262 Hz) gets ~+1.5 dB, and A4 (440 Hz) gets ~+0.3 dB.

Effects are just pipe stages -- stack them freely:

```sc
voice lead = saw(G5) >> lowpass(1200, 0.7) >> delay(0.3, 0.4, 0.3) >> reverb(0.6, 0.5, 0.25)
```

### Parametric EQ

Three band types for surgical frequency shaping:

```sc
// Peak (bell): boost or cut at a center frequency with Q bandwidth
saw(C3) >> eq(400, -3, 1.5)       // cut 3 dB at 400 Hz, Q=1.5 (narrow)
saw(C3) >> eq(3000, 2, 0.8)       // boost 2 dB at 3 kHz, Q=0.8 (wide)

// Low shelf: boost or cut everything below the corner frequency
saw(C2) >> eq(80, 4, low)         // +4 dB bass warmth
saw(C2) >> eq(200, -3, low)       // cut sub-bass mud

// High shelf: boost or cut everything above the corner frequency
sine(A4) >> eq(10000, 2, high)    // +2 dB air/sparkle
saw(C4) >> eq(8000, -4, high)     // tame harshness
```

Stack multiple bands into an fx chain for multi-band EQ:

```sc
fx master_eq = eq(80, 3, low) >> eq(400, -2, 1.5) >> eq(3000, 2, 0.8) >> eq(12000, 2, high)
fx radio = highpass(300, 0.5) >> lowpass(3000, 0.3) >> eq(1000, 4, 0.6)
```

Q values for peak bands: 0.5 = very wide (gentle, broad), 1.0 = moderate (default), 3.0+ = narrow/surgical. Shelves use a fixed Q of 0.707 (Butterworth, maximally flat).

### Sidechain Compression

Duck one signal based on another signal's level -- the classic EDM/house "pumping" effect where pads or bass duck on every kick hit:

```sc
voice kick = sine(55) >> decay(15)
voice pad = chord(Cm7) >> lowpass(800, 0.6)

// Tag the kick's output to the "drums" bus
at 0 play kick >> bus(drums) for 0.5 beats

// The pad ducks whenever the "drums" bus is loud
at 0 play pad >> sidechain(drums, -20, 4, 0.01, 0.1) for 8 beats
```

`bus(name)` tags an event's audio output so other events can react to it. `sidechain(bus, threshold, ratio, attack, release)` applies gain reduction when the named bus exceeds the threshold.

Parameters:

- `threshold` -- dB level above which compression kicks in (e.g. -20)
- `ratio` -- compression ratio (e.g. 4 = 4:1 reduction)
- `attack` -- how fast the ducker responds (seconds, e.g. 0.01)
- `release` -- how fast it recovers (seconds, e.g. 0.1)

Only `bus` is required -- `sidechain` defaults to -20 dB, 4:1, 10ms attack, 100ms release if parameters are omitted:

```sc
pad >> sidechain(drums)                     // defaults
pad >> sidechain(drums, -20, 4, 0.01, 0.1) // explicit
```

## Effect Chains (`fx`)

Name a reusable pipeline of effects -- like a guitar pedal board:

```sc
fx hall = reverb(0.8, 0.4, 0.35) >> delay(0.3, 0.2, 0.15)
fx telephone = highpass(300, 0.5) >> lowpass(2000, 0.3) >> distort(3.0)
fx tape = chorus(0.015, 0.008, 0.2) >> distort(1.2)

voice pad = chord(Cm7) >> lowpass(800, 0.6) >> hall
at 0 play sine(A4) >> telephone for 4 beats
```

An `fx` is a named chain of transforms with no signal source. Insert it anywhere in a pipe chain. Multiple voices can share the same `fx` for consistent processing.

## Instruments

Define a signal chain once, play it at any pitch. Use `freq` as a variable -- it gets substituted with the actual Hz value when you invoke the instrument:

```sc
instrument piano = ((0.45 * saw(freq) >> lowpass(freq * 4, 0.7) >> decay(8))
  + (1.8 * saw(freq) + 0.35 * saw(freq * 2))
  >> lowpass(freq * 1.2, 0.6) >> chorus(0.016, 0.006, 0.1))
  >> decay(2.0) >> reverb(0.6, 0.3, 0.2)

at 0 play piano(C4) for 4 beats
at 0 play piano(Ab3) >> swell(0.0, 0.5) for 4 beats
```

`freq` works anywhere in the expression -- oscillator arguments, filter cutoffs (`freq * 4`), arithmetic (`freq * 2` for octave-up harmonics). Constant expressions are folded at substitution time.

Instruments compose with everything: pipe into `fx` chains, use with `swell`, and the arp uses `substitute_var` so filter tracking is preserved across all arpeggiated notes.

### Multi-note Instrument Calls

Instruments can accept multiple frequencies, producing a summed chord:

```sc
// Play a chord with an instrument
at 0 play piano(C4, E4, G4) for 4 beats
```

The engine instantiates the instrument's signal chain once per frequency, scales each by `1/N`, and sums them.

### Volume Normalization

Different instruments produce different output levels depending on their synthesis chain. `normalize` levels them to a consistent volume:

```sc
instrument bass = sine(freq) >> lowpass(freq * 3, 0.5) >> decay(12)
instrument piano = saw(freq) >> lowpass(freq * 4, 0.7) >> decay(8)

normalize bass 0.5
normalize piano 0.5
```

The target is on a 0.0-1.0 scale where 1.0 = full scale (0 dBFS) and 0.5 = comfortable level (-6 dB). The engine renders short test tones at multiple frequencies (C2 through C6) through the instrument, measures the average RMS, and applies a gain correction. After normalization, `bass` and `piano` produce comparable output regardless of their synthesis chains.

## Chords

`chord(name)` generates a summed set of saw oscillators for a named chord. Use it anywhere you'd use an oscillator:

```sc
voice pad = chord(Cm7) >> lowpass(800, 0.6) >> reverb(0.7, 0.5, 0.2)
voice bright = chord(Fmaj7) >> chorus(0.012, 0.004, 0.2)
```

### Supported Chord Types

| Suffix | Chord | Intervals |
|--------|-------|-----------|
| `maj` | Major triad | root, 3, 5 |
| `m` / `min` | Minor triad | root, b3, 5 |
| `dim` | Diminished | root, b3, b5 |
| `aug` | Augmented | root, 3, #5 |
| `7` / `dom7` | Dominant 7th | root, 3, 5, b7 |
| `maj7` | Major 7th | root, 3, 5, 7 |
| `m7` / `min7` | Minor 7th | root, b3, 5, b7 |
| `m7b5` | Half-diminished | root, b3, b5, b7 |
| `dim7` | Diminished 7th | root, b3, b5, bb7 |
| `aug7` | Augmented 7th | root, 3, #5, b7 |
| `mmaj7` | Minor-major 7th | root, b3, 5, 7 |
| `9` / `dom9` | Dominant 9th | root, 3, 5, b7, 9 |
| `maj9` | Major 9th | root, 3, 5, 7, 9 |
| `m9` / `min9` | Minor 9th | root, b3, 5, b7, 9 |
| `add9` | Major add 9 | root, 3, 5, 9 |
| `6` | Major 6th | root, 3, 5, 6 |
| `m6` | Minor 6th | root, b3, 5, 6 |
| `sus2` | Suspended 2nd | root, 2, 5 |
| `sus4` | Suspended 4th | root, 4, 5 |

The root is any note letter (A-G) with optional accidental (`#`, `s`, `b`). Append a single digit for octave (default 4): `Abmaj73` = Ab major 7th at octave 3, `Cm7` = C minor 7th at octave 4.

**G7 gotcha:** `G7` is parsed as the note G in octave 7, not a G dominant 7th chord. Use `Gdom7` for the chord.

## Arpeggiator

The arpeggiator splits a voice into a sequence of notes over time. It lives in the pipe chain, so downstream effects apply to all notes. You can spell out individual notes or use chord names:

```sc
voice pluck = 0.3 * saw(0) >> lowpass(2000, 0.8) >> decay(10)

// Chord shorthand -- Cm7 expands to C4, Eb4, G4, Bb4
at 0 play pluck >> arp(Cm7, 4) >> lowpass(1500, 0.6) for 4 beats

// Or spell out individual notes
at 0 play pluck >> arp(C4, Eb4, G4, Bb4, 4) >> lowpass(1500, 0.6) for 4 beats
```

Format: `arp(notes..., rate, options...)` -- notes are frequencies or chord names, rate is notes per beat, and options control direction, octaves, gate, accent, steps, and speed.

Works with voices, instruments, and wavetables: `pluck >> arp(...)`, `piano >> arp(...)`, `piano(0) >> arp(...)` all work. The arp substitutes its own frequencies regardless of what the template was given.

### Direction and Octave Spanning

```sc
pluck >> arp(Cm7, 4)              // ascending (default)
pluck >> arp(Cm7, 4, down)        // descending
pluck >> arp(Cm7, 4, updown)      // ping-pong (up then down)
pluck >> arp(Cm7, 4, random)      // random note each step

// Octave spanning -- play across multiple octaves before repeating
pluck >> arp(Cm7, 4, up2)         // ascending across 2 octaves
pluck >> arp(Cm7, 4, down3)       // descending across 3 octaves
pluck >> arp(Cm7, 4, updown2)     // ping-pong across 2 octaves
```

### Gate Length

Controls note duration relative to step length. Default is 1.0 (full step). Less than 1.0 creates staccato, greater than 1.0 creates legato overlap:

```sc
pluck >> arp(Cm7, 4, gate, 0.5)              // staccato (50% of step)
pad >> arp(Cm7, 2, updown, gate, 1.5)        // legato (notes overlap)
```

### Accent Pattern

Boosts every Nth note (1.5x gain on accented, 0.7x on unaccented):

```sc
pluck >> arp(Cm7, 8, accent, 4)              // accent every 4th note
pluck >> arp(Cm7, 8, down, accent, 3)        // descending, accent every 3rd
```

### Step Pattern

Rhythmic gating -- `x` plays, `_` rests. The pattern cycles:

```sc
pluck >> arp(Cm7, 8, steps, x_x_xx_x)       // rhythmic pattern
pluck >> arp(Cm7, 8, updown, steps, xxx_)    // 3 on, 1 off
```

### Speed Ramp

Uses the range syntax (`->`) for the rate to accelerate or decelerate:

```sc
pluck >> arp(Cm7, 2 -> 8) for 8 beats       // accelerate: 2 to 8 notes/beat
pluck >> arp(Cm7, 8 -> 2) for 8 beats       // decelerate
pluck >> arp(Cm7, 2 -> 8, updown) for 8 beats  // ramp + direction
```

### Combining Options

Options can be combined freely after the rate:

```sc
pluck >> arp(Cm7, 8, updown2, gate, 0.3, accent, 4, steps, x.xx) for 8 beats
```

## Operators

| Operator | Meaning | Example |
|---|---|---|
| `>>` | Chain -- output of left feeds into right | `saw(C3) >> lowpass(800, 0.7)` |
| `+` | Mix -- add signals together | `sine(A4) + sine(A5)` |
| `-` | Subtract signals | `sine(A4) - sine(A5)` |
| `*` | Scale -- multiply by a number | `0.5 * sine(A4)` (half volume) |
| `/` | Divide -- useful in instruments | `200 / freq` (inverse frequency scaling) |
| `->` | Sweep -- linear interpolation over event duration | `lowpass(800 -> 4000, 0.7)` |

Parentheses group sub-expressions: `(saw(C3) + sine(C4)) >> lowpass(1000, 1.0)`

Operator precedence (highest to lowest): `*` `/`, `+` `-`, `>>`.

## Voice Substitution (`with`)

Patterns and sections use voice names as placeholders. The `with` clause lets you swap in different instruments at play-time -- the same drum pattern works with any kit, the same melody works with any instrument.

Three levels of scoping (innermost wins):

```sc
// Global defaults -- apply to everything
with kick = analog_kick, snare = tight_snare, hat = crispy_hat

// Section-level -- override globals for this section
section verse = 16 beats with {kick = 808_kick, snare = clap}
  repeat drums every 4 beats                           // uses section defaults
  repeat drums every 4 beats with {hat = shaker}       // override just the hat

// Per-entry -- override for one specific use
  play melody_line with {mel = rhodes}
```

This decouples rhythm/melody from timbre. A boom-bap pattern defined with `kick`, `snare`, and `hat` works with electronic drums, acoustic samples, or synthesized percussion -- just change the `with` bindings.
