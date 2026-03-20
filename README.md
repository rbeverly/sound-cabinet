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

The binary will be at `target/release/sound-cabinet`. Either copy it to a system directory or add the build output to your shell's PATH:

```bash
# Option A: copy to a system directory
cp target/release/sound-cabinet /usr/local/bin/

# Option B: add to your PATH (run from the sound-cabinet directory)

# bash (~/.bashrc)
echo 'export PATH="$PATH:'$(pwd)'/target/release"' >> ~/.bashrc && source ~/.bashrc

# zsh (~/.zshrc)
echo 'export PATH="$PATH:'$(pwd)'/target/release"' >> ~/.zshrc && source ~/.zshrc

# fish (~/.config/fish/config.fish)
echo 'fish_add_path '(pwd)'/target/release' >> ~/.config/fish/config.fish && source ~/.config/fish/config.fish
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

# Watch mode — live reload on file save
sound-cabinet watch examples/demo.sc

# Piano mode — play any instrument live with your keyboard
sound-cabinet piano examples/voices/concerto2-kit.sc piano

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
| `pulse(C3, 0.25)` | Pulse wave — variable width (0.0-1.0). 0.5 = square, 0.1 = thin/nasal, 0.9 = bright. |
| `noise()` | White noise (no frequency argument) |

Pulse width can be swept over time using parameter automation: `pulse(C3, 0.1 -> 0.9)` — classic PWM synth pad sound.

#### Custom waveforms

Define arbitrary waveform shapes as arrays of sample points. The array represents one cycle — the oscillator reads through it at the right speed for the frequency, interpolating linearly:

```
wave plateau = [0.0, 0.4, 0.8, 1.0, 1.0, 1.0, 0.8, 0.4, 0.0, -0.4, -0.8, -1.0, -1.0, -1.0, -0.8, -0.4]
wave spike = [0.0, 1.0, 0.3, 0.1, 0.0, -0.1, -0.3, -1.0]

at 0 play 0.3 * plateau(C3) >> lowpass(2000, 0.7) for 4 beats
at 4 play 0.3 * spike(A4) >> reverb(0.6, 0.4, 0.25) for 4 beats
```

Array length determines resolution. Fewer points = crunchier, more aliased (8-bit character). More points = smoother, higher fidelity. Waves don't need to be symmetric — asymmetry adds even harmonics (tube/tape warmth). Custom waves work with instruments, effects, arp — everything in the pipe chain.

### Filters

Process an incoming signal. Chain after a source with `>>`:

| Function | Effect |
|---|---|
| `lowpass(freq, q)` | Cuts frequencies above `freq`. `q` controls resonance (0.5 = gentle, 2.0 = sharp peak). |
| `highpass(freq, q)` | Cuts frequencies below `freq`. Same `q` behavior. |

#### Parameter automation

Filter frequencies (and pulse width) can sweep over time using the `->` operator:

```
saw(C3) >> lowpass(800 -> 4000, 0.7)     // filter opens over event duration
saw(C3) >> highpass(200 -> 50, 0.5)      // highpass sweeps down
pulse(C3, 0.1 -> 0.9)                    // pulse width modulation
```

The sweep is linear from start to end over the duration of the enclosing `for N beats` event.

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
| `compress(thresh, ratio, atk, rel)` | Dynamic range compression. `thresh` in dB, `ratio` e.g. 4 = 4:1, `atk`/`rel` in seconds. | `saw(C2) >> compress(-15, 4, 0.01, 0.1)` |

Effects are just pipe stages — you can stack them freely:

```
voice lead = saw(G5) >> lowpass(1200, 0.7) >> delay(0.3, 0.4, 0.3) >> reverb(0.6, 0.5, 0.25)
```

### Effect Chains (`fx`)

Name a reusable pipeline of effects — like a guitar pedal board:

```
fx hall = reverb(0.8, 0.4, 0.35) >> delay(0.3, 0.2, 0.15)
fx telephone = highpass(300, 0.5) >> lowpass(2000, 0.3) >> distort(3.0)
fx tape = chorus(0.015, 0.008, 0.2) >> distort(1.2)

voice pad = chord(Cm7) >> lowpass(800, 0.6) >> hall
at 0 play sine(A4) >> telephone for 4 beats
```

An `fx` is a named chain of transforms with no signal source. Insert it anywhere in a pipe chain. Multiple voices can share the same `fx` for consistent processing.

### Instruments

Define a signal chain once, play it at any pitch. Use `freq` as a variable — it gets substituted with the actual Hz value when you invoke the instrument:

```
instrument piano = ((0.45 * saw(freq) >> lowpass(freq * 4, 0.7) >> decay(8)) + (1.8 * saw(freq) + 0.35 * saw(freq * 2)) >> lowpass(freq * 1.2, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> reverb(0.6, 0.3, 0.2)

at 0 play piano(C4) for 4 beats
at 0 play piano(Ab3) >> swell(0.0, 0.5) for 4 beats
```

`freq` works anywhere in the expression — oscillator arguments, filter cutoffs (`freq * 4`), arithmetic (`freq * 2` for octave-up harmonics). Constant expressions are folded at substitution time.

Instruments compose with everything: pipe into `fx` chains, use with `swell`, and arp uses `substitute_var` so filter tracking is preserved across all arpeggiated notes.

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

### Sustain Pedal

The sustain pedal extends notes beyond their key-down duration, simulating piano damper behavior:

```
pedal down at 4.0
at 4.0 play piano(C4) for 1 beat    // note rings until pedal up
at 4.5 play piano(E4) for 1 beat    // also sustained
pedal up at 8.0                      // both notes released
```

Notes that end while the pedal is down have their duration extended to the pedal-up point. The MIDI converter (`midi2sc.py`) automatically translates CC64 pedal events into `pedal down/up` instructions.

### Swing & Humanize

Timing transforms that make patterns feel human. Swing shifts offbeat events (eighth-note positions like 0.5, 1.5, 2.5) later within each beat. Humanize adds random timing jitter.

**Global** — applies to all patterns that don't have their own swing/humanize:

```
swing 0.62        // 0.5 = straight, 0.67 = triplet swing
humanize 8        // ±8ms random jitter per event
```

**Per-pattern** — overrides global settings for that pattern:

```
pattern hats = 4 beats swing 0.65 humanize 5
  at 0.5 play hat for 0.2 beats
  at 1.5 play hat for 0.2 beats

pattern kick = 4 beats
  at 0 play kick for 0.5 beats    // straight — no swing
```

This lets you swing the hats while keeping the kick on the grid, or humanize the melody while leaving the drums robotic.

### Operators

| Operator | Meaning | Example |
|---|---|---|
| `>>` | Chain — output of left feeds into right | `saw(C3) >> lowpass(800, 0.7)` |
| `+` | Mix — add signals together | `sine(A4) + sine(A5)` |
| `-` | Subtract signals | `sine(A4) - sine(A5)` |
| `*` | Scale — multiply by a number | `0.5 * sine(A4)` (half volume) |
| `/` | Divide — useful in instruments | `200 / freq` (inverse frequency scaling) |
| `->` | Sweep — linear interpolation over event duration | `lowpass(800 -> 4000, 0.7)` |

Parentheses group sub-expressions: `(saw(C3) + sine(C4)) >> lowpass(1000, 1.0)`

Operator precedence (highest to lowest): `*` `/`, `+` `-`, `>>`.

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

## Piano Mode

Play any instrument or custom waveform live with your keyboard:

```bash
sound-cabinet piano examples/voices/concerto2-kit.sc piano
sound-cabinet piano examples/wave-test.sc spike
sound-cabinet piano examples/voices/lofi-kit.sc mel
```

The first argument is a score file (loads its instrument/voice/fx/wave definitions). The optional second argument is the instrument or wave name to play. Without it, a default sine+decay tone is used.

The keyboard maps two chromatic octaves (C3–C5) across your QWERTY layout — the same layout as GarageBand. Press Esc or Ctrl+C to exit.

## Examples

The `examples/` directory includes several complete compositions:

| File | Description |
|---|---|
| `demo.sc` | Basic features walkthrough |
| `effects-demo.sc` | Showcases effects, arp, pulse oscillator, PWM sweep, filter automation, and compression |
| `concerto2.sc` | Rachmaninoff Piano Concerto No. 2 (converted from MIDI) |
| `lofi-afternoon.sc` | Lofi hip-hop track with swing, chorus, distortion, and vibrato |
| `wave-test.sc` | Custom waveform demo — plateau, spike, asymmetric, ziggurat |
| `compress-test.sc` | A/B comparison of compression on drums, bass, and pads |

Voice kits in `examples/voices/` define reusable instrument sets that compositions import.

Render any example:

```bash
sound-cabinet render examples/effects-demo.sc -o effects-demo.wav
sound-cabinet render examples/lofi-afternoon.sc -o lofi-afternoon.wav
```

## Roadmap

What's coming next, roughly in priority order.

### Waveshaping modes

Extend `distort` with named modes beyond the current symmetric tanh soft-clip — asymmetric clipping (tube warmth), foldback distortion (aggressive harmonics), half-wave rectification (even harmonics):

```
saw(C3) >> distort(3.0, "fold")    // foldback
sine(A4) >> distort(2.0, "asym")   // asymmetric / tube-style
```

### Wavetable interpolation modes

Custom waveforms currently use linear interpolation between sample points. Non-linear modes (cubic, spline) would allow smooth curves with fewer points — a 4-point wave with cubic interpolation could produce a bell curve that needs 64+ points linearly. Specified as a per-wave argument:

```
wave bell cubic = [0.0, 1.0, 1.0, 0.0]    // cubic interpolation
wave harsh = [0.0, 1.0, -1.0, 0.0]         // default: linear
```

### Wave grid syntax

Visual grid definition for waveforms (rows = amplitude, columns = time):

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

### Multi-cycle waveforms

Compose multiple wave definitions into a longer repeating pattern. The fundamental period becomes the full sequence, creating richer harmonics:

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

### Composable timing transforms (future)

Play-time piping for swing — apply different swing to different layers in the same section:

```
section groove = 16 beats
  repeat hats every 4 beats >> swing 0.7
  repeat kick_pattern every 4 beats
  play bass
```

Also: expressive dynamics (`rush`, `drag`, `push`) for manual performance markup, and algorithmic humanization based on musical structure heuristics.

### Velocity & dynamics

Per-note velocity so drum patterns and melodies feel human instead of mechanical:

```
at 0 play kick vel 0.9 for 0.5 beats
at 1 play snare vel 0.6 for 0.25 beats
```

### Sidechain compression

Compress one signal based on another signal's level — the classic EDM/house "pumping" effect where the bass ducks on every kick hit:

```
voice bass = saw(C2) >> lowpass(400, 1.0) >> sidechain(kick, -20, 4, 0.001, 0.1)
```

Requires routing one signal's envelope to control another signal's gain — a more complex architecture than the current per-voice compressor.

### Sostenuto pedal

Selective sustain — only holds notes that are already pressed when the pedal goes down. New notes played after are damped normally. Used for holding a bass note while playing staccato melody above it:

```
sostenuto down at 4.0    // captures currently-sounding notes
sostenuto up at 8.0      // releases only those notes
```

Requires per-note tracking of which events were active at pedal-down time, unlike the sustain pedal which extends all notes blindly.

### Una corda (soft pedal)

Shifts the piano hammer mechanism to strike fewer strings — quieter and timbrally darker, not just a volume reduction. In the engine this would apply a gain reduction + lowpass filter shift to all notes while the pedal is down:

```
soft down at 4.0
soft up at 8.0
```

### EQ (parametric equalizer)

Multi-band parametric EQ as a pipe-chain effect. Boost or cut specific frequency ranges — essential for mixing and for solving the equal-loudness problem in instrument definitions:

```
// 3-band EQ: boost bass, cut muddy mids, add treble air
fx master_eq = eq(80, 6, "shelf") >> eq(400, -3, 1.0) >> eq(10000, 3, "shelf")

// In an instrument: compensate for Fletcher-Munson curve
instrument piano = ... >> eq(80, 8, "shelf") >> eq(200, 4, 1.0)
```

Also useful for shaping individual voices, creating telephone/radio effects, or matching the tonal character of reference tracks.

### Master output / distribution-ready export

Post-processing pipeline for the final mix: peak normalization, loudness targeting (LUFS), optional limiting, and export to distribution-ready formats. The goal is to go from `.sc` to DistroKid-ready without leaving Sound Cabinet:

```bash
sound-cabinet render track.sc -o track.wav --normalize --lufs -14 --format mp3
```

This includes: peak/RMS normalization, loudness metering (integrated LUFS per streaming platform targets), a brickwall limiter to prevent clipping, and format conversion (MP3, AAC, FLAC).

### Fletcher-Munson equal-loudness compensation

Built-in frequency-dependent gain compensation that models human hearing sensitivity. Low frequencies need significantly more energy to sound equally loud as midrange — this curve is logarithmic, not linear. A dedicated `loudness()` function in instruments would apply the ISO 226 equal-loudness contour automatically:

```
instrument piano = ... >> loudness(freq)   // auto-compensates based on pitch
```

This would replace the manual `200 / freq` approximation with a proper psychoacoustic curve.

### MIDI export (sc2midi)

Render to `.mid` instead of `.wav` so compositions can be brought into a DAW with real instruments. The arp and note-name infrastructure already maps cleanly to MIDI events. Combined with the existing `midi2sc.py` importer, this creates a round-trip: MIDI → .sc (edit/compose) → MIDI (produce in DAW).

### MIDI keyboard support

Connect a physical MIDI keyboard for live playing with velocity sensitivity, pedal support, and mod wheel. Uses the `midir` crate to listen for MIDI note-on/note-off events. Would enable real velocity values (instead of uniform volume) and CC data (sustain pedal, expression).

### Note-on / note-off engine support

The engine currently schedules notes with fixed durations. For realistic live playing, instruments need two distinct behaviors:

- **Percussive** (piano, plucked strings): key down fires a single impulse, note decays naturally, key up activates damper (fast fade)
- **Sustained** (organ, synth pad): key down starts continuous generation, key up stops it with a release envelope

This requires adding a note-off event type to the engine's scheduling system, enabling proper key-duration-sensitive playback in piano mode and MIDI input.

### VST3/AU plugin export

Compile Sound Cabinet instruments and effect chains into native DAW plugins (VST3 for cross-platform, Audio Unit for Logic/GarageBand). The Rust `nih-plug` framework provides the plugin host wrapper — the core work is packaging a fundsp signal graph as a plugin that accepts MIDI input and produces audio. This would let instruments built in Sound Cabinet run natively inside Logic, Ableton, GarageBand, etc.

