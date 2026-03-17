# Sound Cabinet

A DSL-driven sound synthesis tool. Write compositions in a simple text format, render them to WAV or play them through your speakers in real-time. A streaming mode lets you pipe instructions in line-by-line as they're generated.

## Quick Start

```bash
cargo build --release

# Render a score to WAV
sound-cabinet render examples/demo.sc -o output.wav

# Play a score through speakers
sound-cabinet play examples/demo.sc

# Stream mode — type lines, hear them immediately
sound-cabinet stream
```

## The Score Format

Score files (`.sc`) are plain text. Each line is one instruction. Lines starting with `//` are comments. Blank lines are ignored.

There are three kinds of instructions:

### 1. Set tempo

```
bpm 120
```

Sets the tempo in beats per minute. If omitted, defaults to 120. Must appear before any `at` lines that depend on it.

### 2. Define a voice

```
voice pad = (saw(40) + 0.5 * sine(80)) >> lowpass(2000, 0.7)
```

Names a reusable signal graph. The name (`pad`) can be used in `play` commands later. Voice definitions don't produce sound on their own — they're templates.

### 3. Schedule playback

```
at 0 play pad for 4 beats
at 2 play sine(440) for 1 beat
```

`at <beat>` is when to start (beat 0 is the beginning). `for <N> beats` is the duration. You can play a named voice or an inline expression.

Multiple events can overlap — they're mixed together automatically.

## Building Expressions

Expressions describe signal graphs using three operators and a set of built-in functions.

### Oscillators

Generate a waveform at a given frequency (Hz):

| Function | Sound |
|---|---|
| `sine(440)` | Pure sine wave at 440 Hz |
| `saw(100)` | Sawtooth wave — bright, buzzy |
| `triangle(200)` | Triangle wave — softer than saw |
| `square(60)` | Square wave — hollow, woody |
| `noise()` | White noise (no frequency argument) |

### Filters

Process an incoming signal. Must be chained after an oscillator with `>>`:

| Function | Effect |
|---|---|
| `lowpass(freq, q)` | Cuts frequencies above `freq`. `q` controls resonance (0.5 = gentle, 2.0 = sharp peak). |
| `highpass(freq, q)` | Cuts frequencies below `freq`. Same `q` behavior. |

### Operators

| Operator | Meaning | Example |
|---|---|---|
| `>>` | Chain — output of left feeds into right | `saw(100) >> lowpass(800, 0.7)` |
| `+` | Mix — add signals together | `sine(440) + sine(880)` |
| `*` | Scale — multiply by a number | `0.5 * sine(440)` (half volume) |

Parentheses group sub-expressions: `(saw(40) + sine(80)) >> lowpass(1000, 1.0)`

Operator precedence (highest to lowest): `*`, `+`, `>>`. So `saw(40) + 0.5 * sine(80) >> lowpass(1000, 1.0)` parses as `(saw(40) + (0.5 * sine(80))) >> lowpass(1000, 1.0)`.

## Composition Examples

### A simple tone

```
bpm 120
at 0 play sine(440) for 4 beats
```

### Layered bass

```
voice bass = (saw(55) + 0.5 * sine(110)) >> lowpass(400, 0.7)
bpm 90
at 0 play bass for 8 beats
```

### A sequence

```
bpm 140
at 0 play sine(262) for 1 beat
at 1 play sine(294) for 1 beat
at 2 play sine(330) for 1 beat
at 3 play sine(349) for 1 beat
at 4 play sine(392) for 2 beat
at 6 play sine(349) for 1 beat
at 7 play sine(330) for 1 beat
```

### Overlapping pads

```
voice warm = (saw(65) + 0.3 * triangle(130)) >> lowpass(600, 0.8)
voice high = 0.4 * sine(523) + 0.3 * sine(659)

bpm 80
at 0 play warm for 8 beats
at 4 play high for 4 beats
```

### Noise percussion

```
voice snare = noise() >> highpass(2000, 1.5)
voice kick = sine(60) + 0.5 * sine(30)

bpm 120
at 0 play kick for 0.5 beats
at 1 play snare for 0.25 beats
at 2 play kick for 0.5 beats
at 3 play snare for 0.25 beats
```

## Streaming Mode

```bash
sound-cabinet stream
```

Reads lines from stdin. Each line is parsed and played immediately — `at 0` means "now", `at 1` means "one beat from now". This is the foundation for piping in generated music from another program:

```bash
echo "bpm 120
at 0 play sine(440) for 2 beats" | sound-cabinet stream
```

## Frequency Reference

For composing with standard musical notes:

| Note | Hz | Note | Hz |
|---|---|---|---|
| C3 | 131 | C4 | 262 |
| D3 | 147 | D4 | 294 |
| E3 | 165 | E4 | 330 |
| F3 | 175 | F4 | 349 |
| G3 | 196 | G4 | 392 |
| A3 | 220 | A4 | 440 |
| B3 | 247 | B4 | 494 |
| C5 | 523 | | |
