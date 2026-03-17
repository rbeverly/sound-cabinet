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
voice pad = (saw(40) + 0.5 * sine(80)) >> lowpass(2000, 0.7)
```

Names a reusable signal graph. Voices are templates — they don't produce sound until played.

#### Schedule playback

```
at 0 play pad for 4 beats
at 2 play sine(440) for 1 beat
```

`at <beat>` is when to start (beat 0 = beginning). `for <N> beats` is the duration. Multiple events can overlap — they're mixed together.

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

### Full composition example

```
import voices/lofi-kit.sc

bpm 75

pattern drums = 4 beats
  at 0 play kick for 0.5 beats
  at 1 play snare for 0.25 beats
  at 2 play kick for 0.5 beats
  at 3 play snare for 0.25 beats

pattern chords = 16 beats
  at 0  play chord1 for 4 beats
  at 4  play chord2 for 4 beats
  at 8  play chord3 for 4 beats
  at 12 play chord4 for 4 beats

section intro = 16 beats
  play chords

section groove = 16 beats
  play chords
  repeat drums every 4 beats

play intro
repeat 4 {
  play groove
}
play intro
```

## Building Expressions

Expressions describe signal graphs using operators and built-in functions.

### Oscillators

Generate a waveform at a given frequency (Hz):

| Function | Sound |
|---|---|
| `sine(440)` | Pure sine wave — clean, simple |
| `saw(100)` | Sawtooth — bright, buzzy |
| `triangle(200)` | Triangle — softer than saw |
| `square(60)` | Square — hollow, woody |
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

Decay values for common sounds:

| Sound | Decay | Character |
|---|---|---|
| `decay(8)` | Slow pad release | ~0.3s to near-silence |
| `decay(12)` | Kick drum thump | ~0.25s |
| `decay(15)` | Snare snap | ~0.2s |
| `decay(25)` | Hi-hat "tss" | ~0.12s |
| `decay(40)` | Sharp click | ~0.07s |

Example — a hi-hat with a sharp percussive attack:

```
voice hat = 0.12 * noise() >> highpass(6000, 1.0) >> decay(25)
```

### Operators

| Operator | Meaning | Example |
|---|---|---|
| `>>` | Chain — output of left feeds into right | `saw(100) >> lowpass(800, 0.7)` |
| `+` | Mix — add signals together | `sine(440) + sine(880)` |
| `*` | Scale — multiply by a number | `0.5 * sine(440)` (half volume) |

Parentheses group sub-expressions: `(saw(40) + sine(80)) >> lowpass(1000, 1.0)`

Operator precedence (highest to lowest): `*`, `+`, `>>`.

## Streaming Mode

```bash
sound-cabinet stream
```

Reads lines from stdin. Each line is parsed and played immediately — `at 0` means "now", `at 1` means "one beat from now":

```bash
echo "bpm 120
at 0 play sine(440) for 2 beats" | sound-cabinet stream
```

This is the foundation for generative music — pipe output from an LLM or any program that generates `.sc` lines.

## Frequency Reference

| Note | Hz | Note | Hz | Note | Hz |
|---|---|---|---|---|---|
| C3 | 131 | C4 | 262 | C5 | 523 |
| D3 | 147 | D4 | 294 | D5 | 587 |
| Eb3 | 156 | Eb4 | 311 | Eb5 | 622 |
| E3 | 165 | E4 | 330 | E5 | 659 |
| F3 | 175 | F4 | 349 | F5 | 698 |
| G3 | 196 | G4 | 392 | G5 | 784 |
| Ab3 | 208 | Ab4 | 415 | Ab5 | 831 |
| A3 | 220 | A4 | 440 | A5 | 880 |
| Bb3 | 233 | Bb4 | 466 | Bb5 | 932 |
| B3 | 247 | B4 | 494 | B5 | 988 |
