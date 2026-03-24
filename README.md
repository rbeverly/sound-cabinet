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

On Windows, download `sound-cabinet-x86_64-pc-windows-msvc.zip` from [Releases](https://github.com/rbeverly/sound-cabinet/releases), extract it, and add the folder to your PATH (or move `sound-cabinet.exe` to a directory already in your PATH).

### Build from source

Requires [Rust](https://www.rust-lang.org/tools/install) (1.70+):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh  # if you don't have Rust
git clone https://github.com/rbeverly/sound-cabinet.git
cd sound-cabinet
cargo build --release
```

The binary will be at `target/release/sound-cabinet` (or `target\release\sound-cabinet.exe` on Windows). Either copy it to a system directory or add the build output to your PATH:

```bash
# Option A: copy to a system directory (macOS/Linux)
cp target/release/sound-cabinet /usr/local/bin/

# Option B: add to your PATH (run from the sound-cabinet directory)

# bash (~/.bashrc)
echo 'export PATH="$PATH:'$(pwd)'/target/release"' >> ~/.bashrc && source ~/.bashrc

# zsh (~/.zshrc)
echo 'export PATH="$PATH:'$(pwd)'/target/release"' >> ~/.zshrc && source ~/.zshrc

# fish (~/.config/fish/config.fish)
echo 'fish_add_path '(pwd)'/target/release' >> ~/.config/fish/config.fish && source ~/.config/fish/config.fish
```

On Windows (PowerShell):

```powershell
# Add to your user PATH permanently
$env:PATH += ";$PWD\target\release"
[Environment]::SetEnvironmentVariable("PATH", $env:PATH, "User")
```

#### Platform dependencies

**macOS** — audio support is built in, no extra dependencies needed.

**Linux** — you may need ALSA development libraries for audio output:

```bash
# Debian/Ubuntu
sudo apt install libasound2-dev

# Fedora
sudo dnf install alsa-lib-devel
```

**Windows** — no extra dependencies needed. Audio uses WASAPI (built into Windows).

## Usage

```bash
# Render a score to a WAV file (prints loudness + peak info)
sound-cabinet render examples/demo.sc -o output.wav

# Render and normalize to a loudness target (e.g. Spotify = -14 LUFS)
sound-cabinet render examples/demo.sc -o output.wav --lufs -14

# Play a score through your speakers
sound-cabinet play examples/demo.sc

# Play with verbose output — shows beat positions and pattern names
sound-cabinet play examples/demo.sc -v

# Skip ahead — start playing from beat 140
sound-cabinet play examples/demo.sc --from 140

# Watch mode — live reload on file save
sound-cabinet watch examples/demo.sc

# Piano mode — play any instrument live with your keyboard
sound-cabinet piano examples/voices/concerto2-kit.sc piano

# Stream mode — type lines or pipe them in, hear them immediately
sound-cabinet stream

# Generate phrases from pattern files (algorithmic composition)
sound-cabinet generate \
  --pattern patterns/bass/walking-jazz.yaml \
  --key D --mode dorian \
  --chords "Dm7 G7 Cmaj7 Am7" \
  --voice bass --variations 5 -o generated.sc

# Export sheet music as LilyPond (or PDF if lilypond is installed)
sound-cabinet export song.sc -o song.ly --key Am
sound-cabinet export song.sc -o piano-part.ly --voice piano --from 0 --to 32
sound-cabinet export song.sc -o song.pdf --title "My Song"
```

### Algorithmic generation

The `generate` command composes musical phrases from YAML pattern files. Each pattern defines a reusable musical gesture through layered decomposition: rhythm (note placement), contour (relative pitch motion), and emphasis (dynamics). The generator resolves these against a key, mode, and chord progression to produce concrete `.sc` patterns.

```bash
sound-cabinet generate \
  --pattern patterns/bass/walking-jazz.yaml \
  --key D --mode dorian \
  --chords "Dm7 G7 Cmaj7 Am7" \
  --voice bass \
  --range C2-G3 \
  --variations 5 \
  --seed 42 \
  -o bass-lines.sc
```

| Flag | Required | Description |
|------|----------|-------------|
| `--pattern` | yes | Path to a YAML pattern file |
| `--key` | yes | Root note (C, D, Bb, F#, etc.) |
| `--mode` | yes | Scale mode (major, minor, dorian, mixolydian, blues, etc.) |
| `--chords` | yes | Space-separated chord progression ("Dm7 G7 Cmaj7") |
| `--voice` | yes | Instrument name for the output patterns |
| `--range` | no | Pitch range, e.g. C2-G3 (defaults by type: bass=C2-G3, melody=C4-C6) |
| `--variations` | no | Number of variations to generate (default: 5) |
| `--seed` | no | RNG seed for reproducibility (default: random) |
| `-o` | no | Output file path (default: stdout) |

The output is standard `.sc` with named patterns (`bass_a`, `bass_b`, ...) ready to import and use:

```sc
import generated/bass-lines.sc

section verse = 16 beats
  repeat pick(bass_a, bass_b, bass_c) every 4 beats
```

Starter patterns ship in `patterns/`:

| Pattern | Type | Description |
|---------|------|-------------|
| `bass/walking-jazz` | bass | Quarter-note walking line with chromatic approach |
| `bass/root-fifth-country` | bass | Alternating root and fifth |
| `bass/octave-pulse` | bass | Driving eighth-note pulse on root |
| `melody/question-phrase` | melody | Ascending phrase creating tension |
| `melody/answer-phrase` | melody | Descending phrase resolving to root |
| `accomp/alberti-bass` | accomp | Classical arpeggiated chord pattern |

See [docs/algorithmic-generation.md](docs/algorithmic-generation.md) for the design and how to write your own patterns.

### Sheet music export

Export any `.sc` score as LilyPond notation for printing or sharing with musicians. The exporter extracts pitches, durations, and voice assignments from the expanded score and produces standard LilyPond `.ly` files.

```bash
# Export all voices
sound-cabinet export song.sc -o song.ly --key Am --title "My Song"

# Export one instrument only
sound-cabinet export song.sc -o bass.ly --voice bass --key Am

# Export events from a specific pattern
sound-cabinet export song.sc -o verse.ly --source verse_a

# Export a beat range (e.g., bars 1-8 in 4/4)
sound-cabinet export song.sc -o intro.ly --from 0 --to 32

# Render directly to PDF (requires LilyPond: brew install lilypond)
sound-cabinet export song.sc -o song.pdf --key Am
```

| Flag | Required | Description |
|------|----------|-------------|
| `-o` | yes | Output file (.ly or .pdf) |
| `--key` | no | Key signature (Am, D, Bb, F#m, etc.) |
| `--voice` | no | Export only this voice/instrument |
| `--source` | no | Export only events from this pattern name |
| `--from` | no | Start beat |
| `--to` | no | End beat |
| `--time` | no | Time signature (default: 4/4) |
| `--title` | no | Title for the score header |
| `--format` | no | `lilypond` (default) or `pdf` |

The exporter auto-detects clefs from pitch range (treble for melody, bass for low voices), quantizes timing to the 16th-note grid, fills gaps with rests, and splits notes across bar lines with ties.

## The Score Format

Score files (`.sc`) are plain text. Lines starting with `//` are comments. Blank lines are ignored.

### Basics

#### Set tempo

```
bpm 120
```

Sets beats per minute. Defaults to 120 if omitted.

#### Tempo changes

You can change tempo mid-score. Each `bpm` statement takes effect from that point forward — timing of all subsequent events adjusts to the new tempo:

```
bpm 78
play intro
play verse

bpm 82
play chorus

bpm 78
play bridge

bpm 74
play outro
```

This works because `play` advances a cursor through the score. When the engine hits a new `bpm` line, it records the tempo change at that beat position. Earlier events keep their original timing; later events use the new tempo.

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
| `noise()` / `white()` | White noise — flat spectrum, bright. Hi-hats, snares, breath textures. |
| `pink()` | Pink noise — 3 dB/octave rolloff, warmer. Vinyl crackle, ambient, rain. |
| `brown()` | Brown noise — 6 dB/octave rolloff, deep rumble. Thunder, ocean, room tone. |

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
| `lowpass(freq, q, mix)` | Same, with dry/wet blend. `mix` 0.0 = all dry, 1.0 = fully filtered, 0.5 = half the original leaks through. |
| `highpass(freq, q)` | Cuts frequencies below `freq`. Same `q` behavior. |
| `highpass(freq, q, mix)` | Same, with dry/wet blend. |

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
| `crush(bits)` | Bit depth reduction. 8 = retro, 10 = subtle grit, 4 = destroyed. | `saw(C3) >> crush(8)` |
| `decimate(factor)` | Sample rate reduction. 2 = half rate, 8 = heavy digital dirt. | `sine(A4) >> decimate(4)` |
| `degrade(amount)` | Combined tape/medium degradation (lowpass + decimate + crush + noise). 0.3 = warm, 0.6 = worn tape, 1.0 = destroyed. | `triangle(C4) >> degrade(0.5)` |
| `loudness(freq)` | ISO 226 equal-loudness compensation. Applies frequency-dependent gain so all pitches are perceived at roughly equal loudness. Reference: 1 kHz = 0 dB. | `saw(freq) >> loudness(freq)` |
| `eq(freq, gain, q)` | Parametric EQ — peak (bell) filter. Boost or cut `gain` dB at `freq` Hz with bandwidth `q`. | `saw(C3) >> eq(400, -3, 1.5)` |
| `eq(freq, gain, low)` | Low shelf — boost or cut everything below `freq`. | `saw(C3) >> eq(80, 4, low)` |
| `eq(freq, gain, high)` | High shelf — boost or cut everything above `freq`. | `sine(A4) >> eq(10000, 2, high)` |
| `bus(name)` | Tag this event's output for sidechain detection. | `kick >> bus(drums)` |
| `sidechain(bus, thresh, ratio, atk, rel)` | Duck signal based on a bus level. Classic pumping effect. | `pad >> sidechain(drums, -20, 4, 0.01, 0.1)` |

`loudness(freq)` is most useful inside instrument definitions where `freq` is automatically substituted with the note's Hz value. A C2 (65 Hz) gets ~+8 dB, a C4 (262 Hz) gets ~+1.5 dB, and A4 (440 Hz) gets ~+0.3 dB. This replaces manual hacks like `200/freq` with a proper psychoacoustic curve.

#### Parametric EQ

Three band types for surgical frequency shaping:

```
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

```
fx master_eq = eq(80, 3, low) >> eq(400, -2, 1.5) >> eq(3000, 2, 0.8) >> eq(12000, 2, high)
fx radio = highpass(300, 0.5) >> lowpass(3000, 0.3) >> eq(1000, 4, 0.6)
```

Q values for peak bands: 0.5 = very wide (gentle, broad), 1.0 = moderate (default), 3.0+ = narrow/surgical. Shelves use a fixed Q of 0.707 (Butterworth, maximally flat).

#### Sidechain Compression

Duck one signal based on another signal's level — the classic EDM/house "pumping" effect where pads or bass duck on every kick hit:

```
voice kick = sine(55) >> decay(15)
voice pad = chord(Cm7) >> lowpass(800, 0.6)

// Tag the kick's output to the "drums" bus
at 0 play kick >> bus(drums) for 0.5 beats

// The pad ducks whenever the "drums" bus is loud
at 0 play pad >> sidechain(drums, -20, 4, 0.01, 0.1) for 8 beats
```

`bus(name)` tags an event's audio output so other events can react to it. `sidechain(bus, threshold, ratio, attack, release)` applies gain reduction when the named bus exceeds the threshold. Parameters:

- `threshold` — dB level above which compression kicks in (e.g. -20)
- `ratio` — compression ratio (e.g. 4 = 4:1 reduction)
- `attack` — how fast the ducker responds (seconds, e.g. 0.01)
- `release` — how fast it recovers (seconds, e.g. 0.1)

Only `bus` is required — `sidechain` defaults to `-20 dB, 4:1, 10ms attack, 100ms release` if parameters are omitted:

```
pad >> sidechain(drums)                     // defaults
pad >> sidechain(drums, -20, 4, 0.01, 0.1) // explicit
```

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

Supported chord types:

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

Format: `arp(notes..., rate, options...)` — notes are frequencies or chord names, rate is notes per beat, and options control direction, octaves, gate, accent, steps, and speed.

Works with voices, instruments, and wavetables: `pluck >> arp(...)`, `piano >> arp(...)`, `piano(0) >> arp(...)` all work. The arp substitutes its own frequencies regardless of what the template was given.

#### Direction and octave spanning

```
pluck >> arp(Cm7, 4)              // ascending (default)
pluck >> arp(Cm7, 4, down)        // descending
pluck >> arp(Cm7, 4, updown)      // ping-pong (up then down)
pluck >> arp(Cm7, 4, random)      // random note each step

// Octave spanning — play across multiple octaves before repeating
pluck >> arp(Cm7, 4, up2)         // ascending across 2 octaves
pluck >> arp(Cm7, 4, down3)       // descending across 3 octaves
pluck >> arp(Cm7, 4, updown2)     // ping-pong across 2 octaves
```

#### Gate length

Controls note duration relative to step length. Default is 1.0 (full step). Less than 1.0 creates staccato, greater than 1.0 creates legato overlap:

```
pluck >> arp(Cm7, 4, gate, 0.5)              // staccato (50% of step)
pad >> arp(Cm7, 2, updown, gate, 1.5)        // legato (notes overlap)
```

#### Accent pattern

Boosts every Nth note (1.5x gain on accented, 0.7x on unaccented):

```
pluck >> arp(Cm7, 8, accent, 4)              // accent every 4th note
pluck >> arp(Cm7, 8, down, accent, 3)        // descending, accent every 3rd
```

#### Step pattern

Rhythmic gating — `x` plays, `.` rests. The pattern cycles:

```
pluck >> arp(Cm7, 8, steps, x.x.xx.x)       // rhythmic pattern
pluck >> arp(Cm7, 8, updown, steps, xxx.)    // 3 on, 1 off
```

#### Speed ramp

Uses the range syntax (`->`) for the rate to accelerate or decelerate:

```
pluck >> arp(Cm7, 2 -> 8) for 8 beats       // accelerate: 2 to 8 notes/beat
pluck >> arp(Cm7, 8 -> 2) for 8 beats       // decelerate
pluck >> arp(Cm7, 2 -> 8, updown) for 8 beats  // ramp + direction
```

#### Combining options

Options can be combined freely after the rate:

```
pluck >> arp(Cm7, 8, updown2, gate, 0.3, accent, 4, steps, x.xx) for 8 beats
```

### Voice Substitution (`with`)

Patterns and sections use voice names as placeholders. The `with` clause lets you swap in different instruments at play-time — the same drum pattern works with any kit, the same melody works with any instrument.

Three levels of scoping (innermost wins):

```
// Global defaults — apply to everything
with kick = analog_kick, snare = tight_snare, hat = crispy_hat

// Section-level — override globals for this section
section verse = 16 beats with {kick = 808_kick, snare = clap}
  repeat drums every 4 beats                           // uses section defaults
  repeat drums every 4 beats with {hat = shaker}       // override just the hat

// Per-entry — override for one specific use
  play melody_line with {mel = rhodes}
```

This decouples rhythm/melody from timbre. A boom-bap pattern defined with `kick`, `snare`, and `hat` works with electronic drums, acoustic samples, or synthesized percussion — just change the `with` bindings.

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

Play any instrument or custom waveform live with your keyboard or a MIDI controller:

```bash
sound-cabinet piano examples/voices/concerto2-kit.sc piano
sound-cabinet piano examples/wave-test.sc spike
sound-cabinet piano examples/voices/lofi-kit.sc mel

# With MIDI keyboard (auto-detects if connected)
sound-cabinet piano examples/voices/concerto2-kit.sc piano --midi

# Specific MIDI port (by index)
sound-cabinet piano examples/voices/concerto2-kit.sc piano --midi 0
```

The first argument is a score file (loads its instrument/voice/fx/wave definitions). The optional second argument is the instrument or wave name to play. Without it, a default sine+decay tone is used.

The QWERTY keyboard maps two chromatic octaves (C3-C5) across your layout — the same as GarageBand. A MIDI keyboard provides the full range with velocity sensitivity. If a MIDI device is connected, it's auto-detected — both keyboard and MIDI work simultaneously.

#### Recording

Capture what you play as `.sc` patterns with timing and velocity:

| Key | Action |
|-----|--------|
| F1 | Start/stop recording (with metronome click) |
| F2 | Save recording to `recorded_N.sc` |
| F3 | Discard current recording |
| Esc | Quit piano mode |

While recording, a metronome click sounds on each beat. Notes are timestamped and saved with beat offsets relative to the BPM. The output is a standard `.sc` pattern you can import into a score.

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
| `instrument-demo.sc` | Showcase of the default instrument library — keys, strings, mallets, bass, pads |
| `with-demo.sc` | Voice substitution demo — same patterns played with different instrument sets |
| `black-glass.sc` | Downtempo electronic with organic textures and layered percussion |
| `neon-cascade.sc` | Progressive house with filter sweeps, supersaw leads, and long builds |
| `three-faces.sc` | Classical theme (Rachmaninoff) reinterpreted as jazz, ragtime, and drum & bass |

Voice kits in `examples/voices/` define reusable instrument sets that compositions import. The **default instrument library** (`voices/instruments.sc`) includes 20+ instruments across 5 families (keys, plucked strings, pads, bass, mallets) plus texture voices (vinyl crackle, tape hiss, room tone) and effect chains (lofi, hall, radio).

## Master Bus

Every render passes through an automatic master bus chain:

1. **Highpass at 30 Hz** — removes inaudible sub-bass that eats headroom (Butterworth 2nd-order)
2. **Lowpass at 18 kHz** — removes ultrasonic content from aliasing and filter resonance (Butterworth 2nd-order)
3. **RMS compressor** — reduces crest factor (the gap between peak transients and sustained content), raising perceived loudness
4. **Brick-wall limiter at -0.3 dBFS** — prevents peaks from hitting 0 dBFS, with 5ms lookahead for clean transient handling

This runs on all output — `render`, `play`, `watch`, `piano`, and `stream`. The master bandpass reclaims headroom stolen by inaudible frequencies, the compressor tightens dynamics, and the limiter catches peaks.

### Master bus configuration

Control the compressor and limiter from within a score:

```
master compress 0.5                // gentle — subtle dynamic tightening
master compress 1.0                // default — standard mastering compression
master compress 2.0                // heavy — loud, punchy, reduced dynamic range
master compress 0                  // off — bypass compressor entirely
master compress -18 2              // explicit threshold (dB) and ratio
master compress -18 2 0.05 0.2     // threshold, ratio, attack (s), release (s)
master ceiling -1.0                // set limiter ceiling to -1.0 dBFS (default: -0.3)
```

Or from the CLI (overrides score settings):

```bash
sound-cabinet render track.sc -o track.wav --compress 2.0
sound-cabinet render track.sc -o track.wav --compress -18,2,0.05,0.2
sound-cabinet render track.sc -o track.wav --ceiling -1.0
sound-cabinet render track.sc -o track.wav --compress 0 --lufs -14  # no compression, LUFS normalization only
```

The compression `amount` maps to threshold/ratio internally: 0.5 = gentle (-36 dB, 1.5:1), 1.0 = standard (-18 dB, 2:1), 2.0 = heavy (-9 dB, 3:1). Higher values produce louder, more compressed output at the cost of dynamic range.

For full control, specify threshold (dB), ratio, and optionally attack/release (seconds): `master compress -18 2 0.05 0.2`. A slow attack (50–100ms) lets transients punch through before compression engages.

### Loudness measurement

Every `render` prints integrated loudness (LUFS, ITU-R BS.1770) and true peak:

```
$ sound-cabinet render examples/lofi-afternoon.sc -o lofi.wav
  Integrated loudness: -15.6 LUFS
  True peak: -0.2 dBFS
Rendered to lofi.wav
```

### Loudness normalization

Use `--lufs` to auto-normalize to a target loudness. Common targets:

| Platform | Target |
|---|---|
| Spotify | -14 LUFS |
| Apple Music | -16 LUFS |
| YouTube | -14 LUFS |
| Broadcast (EBU R128) | -23 LUFS |

```bash
sound-cabinet render track.sc -o track.wav --lufs -14
```

The normalizer applies gain after rendering to hit the target. If the resulting peak would exceed -0.1 dBFS, it warns about clipping risk.

Render any example:

```bash
sound-cabinet render examples/effects-demo.sc -o effects-demo.wav
sound-cabinet render examples/lofi-afternoon.sc -o lofi-afternoon.wav
```

## Roadmap

What's coming next, roughly in priority order.

### Expression ranges

The `->` sweep operator currently only accepts literal numbers (`800 -> 4000`). Expression ranges would allow freq-relative sweeps inside instruments — essential for filter envelopes that track the note:

```
// Currently not supported — but should be:
instrument pluck = saw(freq) >> lowpass(freq * 8 -> freq * 1.5, 0.6) >> decay(12)
```

This requires extending `Expr::Range` from `Range(f64, f64)` to `Range(Box<Expr>, Box<Expr>)`, and having the graph builder evaluate both sides before constructing the sweep envelope. The key challenge is that `freq * 8` isn't known until instrument instantiation time, so the range evaluation needs to happen after `substitute_var`.

### Parallel signal routing

Named internal buses inside `fx` definitions that allow splitting, processing, and recombining signals. Essential for effects that need to reference the input from multiple processing paths (e.g., replacing high frequencies with noise, wet/dry processing):

```
fx worn_tape = {
  dry: lowpass(1200, 0.3)
  noise: 0.03 * pink() >> highpass(1000, 0.5)
  out: dry + noise
}
```

This enables frequency-dependent noise replacement and any effect where one signal controls another within a single fx chain.

### Import namespacing

Prevent name collisions when importing multiple kits. Currently the second import silently overwrites the first:

```
import voices/lofi-kit.sc as lofi       // lofi.bass, lofi.mel, etc.
import voices/instruments.sc as inst     // inst.rhodes, inst.nylon, etc.
```

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

### Algorithmic phrase generation: ornamentation pass

The base phrase generation system is implemented (`sound-cabinet generate`). What remains is the ornamentation layer: an optional post-processing pass that decorates resolved phrases with mordents, turns, trills, grace notes, and passing tones. Controlled by a density level (0 = clean, 3 = florid/baroque). Each ornament pattern specifies where it can attach (strong beats, long notes, phrase endings) and the generator applies them probabilistically.

See [docs/algorithmic-generation.md](docs/algorithmic-generation.md) for the full design.

### Algorithmic instrument generation

Trait-driven instrument synthesis that builds playable instruments from high-level descriptive vocabulary: `"plucked, decaying, woody, plinky"` resolves to a concrete signal chain (oscillators, filters, envelopes) through archetype templates and trait-to-parameter mapping. A small set of archetypes (plucked string, hammered string, bowed string, blown pipe, struck percussion, electronic) combined with ~30 descriptive traits produces dozens of usable instruments without requiring synthesis knowledge.

See [docs/instrument-generation.md](docs/instrument-generation.md) for the full design.

### True peak limiter

The current `BrickwallLimiter` operates on sample values, but the reconstructed analog signal between samples can peak higher than either sample (inter-sample peaks). The `true_peak_dbfs` measurement detects these, but the limiter can't prevent them. The post-normalization limiter works around this by using a -1.0 dBFS ceiling to leave headroom, but this costs ~0.7 dB of loudness. A proper true peak limiter would oversample the detection stage (typically 4x), detect peaks in the oversampled domain, and apply gain reduction accordingly — matching what professional mastering limiters do.

### Format export (MP3, FLAC, AAC)

The master bus, LUFS measurement, and normalization are implemented. What remains is format conversion for direct upload to streaming platforms:

```bash
sound-cabinet render track.sc -o track.mp3 --lufs -14
```

### Automatic equal-loudness compensation

The `loudness(freq)` function is implemented as an explicit pipe-chain effect. A future enhancement could apply it automatically to all instruments via a global `loudness on` directive or instrument defaults, removing the need to add `>> loudness(freq)` to every instrument definition manually.

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

