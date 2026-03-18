# Sound Cabinet

A DSL-driven sound synthesis tool. Write compositions in a simple text format, render them to WAV, or play them through your speakers in real-time. A streaming mode lets you pipe instructions in line-by-line — designed for both human composers and generative AI.

## Install

### Pre-built binaries

Download the latest release for your platform from [Releases](https://github.com/rbeverly/sound-cabinet/releases):

```bash
# macOS (Apple Silicon)
curl -L https://github.com/rbeverly/sound-cabinet/releases/latest/download/sound-cabinet-aarch64-apple-darwin.tar.gz | tar xz
sudo mv sound-cabinet /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/rbeverly/sound-cabinet/releases/latest/download/sound-cabinet-x86_64-apple-darwin.tar.gz | tar xz
sudo mv sound-cabinet /usr/local/bin/

# Linux (x86_64)
curl -L https://github.com/rbeverly/sound-cabinet/releases/latest/download/sound-cabinet-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv sound-cabinet /usr/local/bin/
```

### Build from source

Requires [Rust](https://www.rust-lang.org/tools/install) (1.70+):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh  # if you don't have Rust
git clone https://github.com/rbeverly/sound-cabinet.git
cd sound-cabinet
cargo build --release
```

The binary will be at `target/release/sound-cabinet`. Copy it somewhere on your `$PATH`:

```bash
cp target/release/sound-cabinet /usr/local/bin/
```

#### Linux dependencies

On Linux you may need ALSA development libraries for audio output:

```bash
# Debian/Ubuntu
sudo apt install libasound2-dev

# Fedora
sudo dnf install alsa-lib-devel
```

macOS has audio support built in — no extra dependencies needed.

## Usage

```bash
# Render a score to a WAV file
sound-cabinet render examples/demo.sc -o output.wav

# Play a score through your speakers
sound-cabinet play examples/demo.sc

# Stream mode — type lines or pipe them in, hear them immediately
sound-cabinet stream
```

## The Score Format

Score files (`.sc`) are plain text. Lines starting with `//` are comments. Blank lines are ignored.

### Basics

#### Set tempo

```
bpm 120
```

Sets beats per minute. Defaults to 120 if omitted.

#### Define a voice

```
voice pad = (saw(C3) + 0.5 * sine(C4)) >> lowpass(2000, 0.7)
```

Names a reusable signal graph. Voices are templates — they don't produce sound until played.

#### Schedule playback

```
at 0 play pad for 4 beats
at 2 play sine(A4) for 1 beat
```

`at <beat>` is when to start (beat 0 = beginning). `for <N> beats` is the duration. Multiple events can overlap — they're mixed together.

### Note Names

Use standard note names instead of raw frequencies. Notes are written as a letter (`A`-`G`), an optional accidental (`#`, `s` for sharp, `b` for flat), and an octave number (`0`-`9`):

```
sine(A4)         // 440 Hz
saw(C4)          // middle C, 261.63 Hz
triangle(Eb3)    // E-flat 3
square(Fs4)      // F-sharp 4 (use 's' instead of '#' if you prefer)
```

Note names work anywhere a frequency is expected — oscillator arguments, arp notes, or any numeric expression.

### Composability

#### Import

Pull in voices and patterns from other files:

```
import voices/lofi-kit.sc
import patterns/drums.sc
```

Paths are relative to the importing file.

#### Pattern

A named, reusable group of events with a duration:

```
pattern boom_bap = 4 beats
  at 0 play kick for 0.5 beats
  at 1 play snare for 0.25 beats
  at 2 play kick for 0.5 beats
  at 3 play snare for 0.25 beats
```

Beat offsets inside a pattern are relative to wherever the pattern is played.

#### Section

Compose patterns together over a duration:

```
section verse = 16 beats
  repeat boom_bap every 4 beats
  play chord_progression
```

`repeat X every N beats` tiles a pattern at regular intervals. `play X` plays a pattern once from the start of the section.

#### Sequential play

At the top level, `play` advances automatically — no need for absolute beat positions:

```
play intro
play verse
play chorus
play outro
```

Each `play` starts after the previous one finishes.

#### Repeat, pick, and shuffle

Loop with variation:

```
repeat 8 {
  pick [verse_a:2, verse_b:2, chorus:1]
}
```

- `pick [a, b, c]` — choose one at random each iteration
- `pick [a:3, b:1]` — weighted random (a is 3x more likely)
- `shuffle [a, b, c]` — play all in random order each iteration
- `play X` — play one specific pattern/section

## Building Expressions

Expressions describe signal graphs using operators and built-in functions. Think of each step as a box — signal flows left to right through the chain.

### Oscillators

Generate a waveform at a given frequency:

| Function | Sound |
|---|---|
| `sine(A4)` | Pure sine wave — clean, simple |
| `saw(C3)` | Sawtooth — bright, buzzy |
| `triangle(E4)` | Triangle — softer than saw |
| `square(G2)` | Square — hollow, woody |
| `noise()` | White noise (no frequency argument) |

### Filters

Process an incoming signal. Chain after a source with `>>`:

| Function | Effect |
|---|---|
| `lowpass(freq, q)` | Cuts frequencies above `freq`. `q` controls resonance (0.5 = gentle, 2.0 = sharp peak). |
| `highpass(freq, q)` | Cuts frequencies below `freq`. Same `q` behavior. |

### Envelopes

Shape amplitude over time:

| Function | Effect |
|---|---|
| `decay(rate)` | Exponential decay: amplitude drops as `e^(-rate * t)`. Higher rate = faster decay. |
| `swell(attack, release)` | Fade in over `attack` seconds, fade out over `release` seconds at the end of the event. |

Decay values for common sounds:

| Sound | Decay | Character |
|---|---|---|
| `decay(8)` | Slow pad release | ~0.3s to near-silence |
| `decay(12)` | Kick drum thump | ~0.25s |
| `decay(15)` | Snare snap | ~0.2s |
| `decay(25)` | Hi-hat "tss" | ~0.12s |
| `decay(40)` | Sharp click | ~0.07s |

### Effects

Effects process a signal in the pipe chain — place them after the source and any filters:

| Function | Effect | Example |
|---|---|---|
| `lfo(rate, depth)` | Tremolo — amplitude modulation. `rate` in Hz, `depth` 0.0-1.0. | `saw(C3) >> lfo(6.0, 0.4)` |
| `distort(amount)` | Soft clipping (tanh saturation). 1.0 = subtle warmth, 4.0+ = heavy drive. | `saw(C2) >> lowpass(400, 1.2) >> distort(4.0)` |
| `vibrato(rate, depth)` | Pitch wobble via modulated delay. `rate` in Hz, `depth` in samples. | `saw(E4) >> vibrato(4.0, 15.0)` |
| `chorus(sep, var, freq)` | Detuned copies for width. `sep`/`var` in seconds, `freq` in Hz. | `triangle(E5) >> chorus(0.015, 0.005, 0.3)` |
| `delay(time, fb, mix)` | Feedback delay. `time` in seconds, `fb` 0.0-1.0 (recirculation), `mix` 0.0-1.0 (dry/wet). Auto-damped HF in feedback path. | `triangle(G5) >> delay(0.3, 0.5, 0.4)` |
| `reverb(size, damp, mix)` | Freeverb algorithmic reverb. `size` 0.0-1.0 (room size), `damp` 0.0-1.0 (HF absorption), `mix` 0.0-1.0 (dry/wet). | `saw(C4) >> reverb(0.8, 0.4, 0.3)` |

Effects are just pipe stages — you can stack them freely:

```
voice lead = saw(G5) >> lowpass(1200, 0.7) >> delay(0.3, 0.4, 0.3) >> reverb(0.6, 0.5, 0.25)
```

### Chords

`chord(name)` generates a summed set of saw oscillators for a named chord. Use it anywhere you'd use an oscillator:

```
voice pad = chord(Cm7) >> lowpass(800, 0.6) >> reverb(0.7, 0.5, 0.2)
voice bright = chord(Fmaj7) >> chorus(0.012, 0.004, 0.2)
```

Supported chord types: `maj`, `m`/`min`, `dim`, `aug`, `7`/`dom7`, `m7`/`min7`, `maj7`, `dim7`, `aug7`, `9`/`dom9`, `m9`/`min9`, `maj9`, `sus2`, `sus4`.

The root is any note letter (A-G) with optional accidental (`#`, `s`, `b`). Octave defaults to 4 but can be specified: `Cm73` for C minor 7th in octave 3.

Note: `G7` is parsed as the note G in octave 7, not a G dominant 7th chord. Use `Gdom7` for the chord.

### Arpeggiator

The arpeggiator splits a voice into a sequence of notes over time. It lives in the pipe chain, so downstream effects apply to all notes. You can spell out individual notes or use chord names:

```
voice pluck = 0.3 * saw(0) >> lowpass(2000, 0.8) >> decay(10)

// Chord shorthand — Cm7 expands to C4, Eb4, G4, Bb4
at 0 play pluck >> arp(Cm7, 4) >> lowpass(1500, 0.6) for 4 beats

// Or spell out individual notes
at 0 play pluck >> arp(C4, Eb4, G4, Bb4, 4) >> lowpass(1500, 0.6) for 4 beats
```

`arp(notes..., speed)` — the last argument is notes per beat. The voice to the left is the template: its oscillator frequencies are replaced by each arp note in turn. Effects to the right of the arp (like `lowpass` above) are applied to every note.

If the voice template has a frequency of `0`, that's fine — the arp substitutes it. If the voice already has a real frequency, the arp overrides it.

### Operators

| Operator | Meaning | Example |
|---|---|---|
| `>>` | Chain — output of left feeds into right | `saw(C3) >> lowpass(800, 0.7)` |
| `+` | Mix — add signals together | `sine(A4) + sine(A5)` |
| `*` | Scale — multiply by a number | `0.5 * sine(A4)` (half volume) |

Parentheses group sub-expressions: `(saw(C3) + sine(C4)) >> lowpass(1000, 1.0)`

Operator precedence (highest to lowest): `*`, `+`, `>>`.

## Streaming Mode

```bash
sound-cabinet stream
```

Reads lines from stdin. Each line is parsed and played immediately — `at 0` means "now", `at 1` means "one beat from now":

```bash
echo "bpm 120
at 0 play sine(A4) for 2 beats" | sound-cabinet stream
```

This is the foundation for generative music — pipe output from an LLM or any program that generates `.sc` lines.

## Examples

The `examples/` directory includes several complete compositions:

| File | Description |
|---|---|
| `demo.sc` | Basic features walkthrough |
| `effects-demo.sc` | Showcases effects, arp, and note names |
| `lofi-afternoon.sc` | Lofi hip-hop track with chorus, distortion, and vibrato |
| `therapy-lofi.sc` | Extended ambient/lofi piece (~4 min) |

Voice kits in `examples/voices/` define reusable instrument sets that compositions import.

Render any example:

```bash
sound-cabinet render examples/effects-demo.sc -o effects-demo.wav
sound-cabinet render examples/lofi-afternoon.sc -o lofi-afternoon.wav
```

## Roadmap

What's coming next, roughly in priority order.

### Pulse oscillator

Variable-width pulse wave — the classic synth waveform that sine/saw/triangle/square can't replicate. Different duty cycles produce dramatically different timbres (thin and nasal at 10%, hollow at 50%, bright at 90%):

```
pulse(C3, 0.25)                        // 25% duty cycle
pulse(C3, 0.1) >> lowpass(800, 0.7)    // narrow pulse, filtered
```

### Waveshaping modes

Extend `distort` with named modes beyond the current symmetric tanh soft-clip — asymmetric clipping (tube warmth), foldback distortion (aggressive harmonics), half-wave rectification (even harmonics):

```
saw(C3) >> distort(3.0, "fold")    // foldback
sine(A4) >> distort(2.0, "asym")   // asymmetric / tube-style
```

### Custom waveforms

Define arbitrary waveform shapes as arrays of sample points. The oscillator interpolates between them and loops at the given frequency:

```
wave wonky = [0.0, 0.3, 0.8, 1.0, 1.0, 0.6, 0.2, -0.5, -0.8, -1.0, -0.4, 0.0]

at 0 play wonky(C3) >> lowpass(800, 0.7) for 4 beats
```

Or as a visual grid (rows = amplitude, columns = time):

```
wave spiky = 8x8 {
  . . . X . . . .
  . . X . X . . .
  . X . . . . . .
  X . . . . . . X
  . . . . . . X .
  . . . . . X . .
  . . . . X . . .
  . . . . . . . .
}
```

Waves don't have to be symmetric — asymmetry adds even harmonics (tube/tape warmth). Multi-cycle patterns are also possible, where the repeating unit is longer than one wave period:

```
wave evolving = cycle [wonky, spiky, spiky, wonky]
```

### Arp enhancements

Direction, octave spanning, and randomization for the arpeggiator:

```
pluck >> arp(Cm7, 4, "down")       // descending
pluck >> arp(Cm7, 4, "updown")     // ascending then descending per cycle
pluck >> arp(Cm7, 4, "random")     // random note order each cycle
pluck >> arp(Cm7, 4, "up2")        // ascend across 2 octaves before repeating
```

### Tuning & microtonal

Change the reference pitch from the default A4=440 Hz:

```
tuning 432        // A4 = 432 Hz — all notes shift accordingly
```

Beyond alternate reference pitches, support non-12-TET tuning systems — 19-TET, 24-TET (quarter tones), just intonation, gamelan pelog/slendro. This changes the fundamental interval math from `2^(semitones/12)` to pluggable tuning tables. Named scale systems (ragas, maqam, pentatonic modes) could work as selections from a tuning: `arp(raga_bhairav, 4)`.

### Swing & humanize

Composable timing transforms — they change *when* events fire, not the audio signal. Can be applied at the pattern definition, at play-time via piping, or globally. Transforms compose multiplicatively when stacked.

```
// On the pattern definition
pattern drums = 4 beats swing 0.6
  at 0 play kick for 0.5 beats
  at 1 play hat for 0.25 beats

// At play-time — different swing per layer in the same section
section groove = 16 beats
  repeat hats every 4 beats >> swing 0.7
  repeat kick_pattern every 4 beats
  play chords >> swing 0.6
  play bass

// Global jitter
humanize 10       // ±10ms per note
```

Swing is the foundation of shuffle, jazz, and boom-bap grooves. Humanize adds the subtle imprecision of a real player. Stacking swing (e.g., swung pattern played with additional swing) pushes notes progressively behind the beat — a real production technique for that Dilla-style loose feel.

### Velocity & dynamics

Per-note velocity so drum patterns and melodies feel human instead of mechanical:

```
at 0 play kick vel 0.9 for 0.5 beats
at 1 play snare vel 0.6 for 0.25 beats
```

### Parameter automation

Sweep any parameter over the duration of an event:

```
saw(C3) >> lowpass(800 -> 4000, 0.7)   // filter opens over time
saw(C3) >> lowpass(800, 0.7) >> lfo(2.0 -> 8.0, 0.4)  // LFO speeds up
```

This is how filter sweeps, risers, and drops work in electronic music.

### MIDI export

Render to `.mid` instead of `.wav` so compositions can be brought into a DAW with real instruments. The arp and note-name infrastructure already maps cleanly to MIDI events.

### Watch mode

Live reload on file save for fast iteration:

```bash
sound-cabinet watch examples/demo.sc
```
