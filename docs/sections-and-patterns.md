[← Back to README](../README.md)

# Patterns, Sections & Composition

This document covers how to structure music in Sound Cabinet: defining reusable patterns, composing them into sections, and arranging sections into a full score.

## Note Names

Use standard note names instead of raw frequencies. Notes are written as a letter (`A`-`G`), an optional accidental (`#`, `s` for sharp, `b` for flat), and an octave number (`0`-`9`):

```sc
sine(A4)         // 440 Hz
saw(C4)          // middle C, 261.63 Hz
triangle(Eb3)    // E-flat 3
square(Fs4)      // F-sharp 4 (use 's' instead of '#' if you prefer)
```

Note names work anywhere a frequency is expected -- oscillator arguments, arp notes, or any numeric expression.

## Patterns

A pattern is a named, reusable group of events with a duration. Beat offsets inside a pattern are relative to wherever the pattern is played:

```sc
pattern boom_bap = 4 beats
  at 0 play kick for 0.5 beats
  at 1 play snare for 0.25 beats
  at 2 play kick for 0.5 beats
  at 3 play snare for 0.25 beats
```

### Swing and Humanize Per-Pattern

Patterns can have their own timing settings that override the global values:

```sc
pattern hats = 4 beats swing 0.65 humanize 5
  at 0.5 play hat for 0.2 beats
  at 1.5 play hat for 0.2 beats

pattern kick = 4 beats
  at 0 play kick for 0.5 beats    // straight -- no swing
```

This lets you swing the hats while keeping the kick on the grid, or humanize the melody while leaving the drums robotic.

## Sections

Compose patterns together over a duration:

```sc
section verse = 16 beats
  repeat boom_bap every 4 beats
  play chord_progression
```

`repeat X every N beats` tiles a pattern at regular intervals. `play X` plays a pattern once from the start of the section.

### Beat Ranges

Tile a pattern only within a specific range using `from`, `to`, and `until`:

```sc
section intro = 32 beats
  repeat clap_funky until 32
  repeat hats_steady from 8 to 32
  repeat marimba_wonky every 4 beats from 16
```

`from` and `to` are independent keywords: use both, either, or neither. `until` is syntactic sugar for `to`. If `every` is omitted, it defaults to the pattern's own duration -- so `repeat hats_steady from 8 to 32` tiles at 1-beat intervals if `hats_steady` is a 1-beat pattern. The implicit `every` is resolved during expansion (when pattern durations are known), not at parse time.

Patterns that extend beyond the section's declared length truncate at the boundary. This is intentional and useful -- playing a 32-beat pattern inside a 16-beat section gives you just the first half:

```sc
section teaser = 16 beats
  at 0 play full_melody    // full_melody is 32 beats, but only the first 16 play
```

### At-Positioning

Start a pattern at a specific beat within a section:

```sc
section verse = 32 beats
  at 8 play fill_pattern
  at 16 repeat hats every 1 beat
```

### Inline Events

Play one-off sounds directly inside a section without defining a separate pattern:

```sc
section verse = 32 beats
  at 0 play sine(440) for 2 beats
  repeat drums every 4 beats
```

### Sequence

Play patterns one after another (sequential, not simultaneous):

```sc
section verse = 32 beats
  repeat drums every 8 beats
  sequence bass_sparse, bass_active    // 16 + 16 = 32, plays back-to-back
```

### Repeat Blocks Inside Sections

Use `repeat N { pick [...] }` and `repeat N { shuffle [...] }` inside sections, not just at the top level:

```sc
section verse = 32 beats
  repeat drums every 8 beats
  repeat 4 {
    pick [groove_a, groove_b]
  }
```

This is semantically identical to the existing top-level `repeat` block -- it just becomes available in more contexts.

### Implicit Section Length

If the beat count is omitted, the section's duration is computed from its contents (the latest endpoint of any contained pattern or repeat):

```sc
section auto_length
  at 0 play 8beatpattern       // ends at beat 8
  at 8 play 32beatpattern      // ends at beat 40
  // section is implicitly 40 beats

// For parsing convenience, "= 0 beats" also signals implicit length:
section auto_length = 0 beats
  at 0 play intro
  at 8 play verse
```

## Sequential Play

At the top level, `play` advances automatically -- no need for absolute beat positions:

```sc
play intro
play verse
play chorus
play outro
```

Each `play` starts after the previous one finishes.

## Repeat Blocks with Pick and Shuffle

Loop with variation at the top level:

```sc
repeat 8 {
  pick [verse_a:2, verse_b:2, chorus:1]
}
```

- `pick [a, b, c]` -- choose one at random each iteration
- `pick [a:3, b:1]` -- weighted random (a is 3x more likely)
- `shuffle [a, b, c]` -- play all in random order each iteration
- `play X` -- play one specific pattern/section

## Tempo Changes

You can change tempo mid-score. Each `bpm` statement takes effect from that point forward:

```sc
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

## Sustain Pedal

The sustain pedal extends notes beyond their key-down duration, simulating piano damper behavior:

```sc
pedal down at 4.0
at 4.0 play piano(C4) for 1 beat    // note rings until pedal up
at 4.5 play piano(E4) for 1 beat    // also sustained
pedal up at 8.0                      // both notes released
```

Notes that end while the pedal is down have their duration extended to the pedal-up point.

### Voice-Scoped Pedal

By default, `pedal down` sustains all voices. Use a voice name to scope it:

```sc
// Only sustain the piano -- drums, bass, etc. are unaffected
pedal down piano at 4.0
pedal up piano at 8.0

// Sustain multiple specific voices
pedal down [piano, strings] at 4.0
pedal up [piano, strings] at 8.0
```

This works with `with` substitutions too. If you have `with lead_piano = piano, rhythm_piano = piano`, you can pedal them independently:

```sc
pedal down lead_piano at 4.0
pedal up lead_piano at 8.0
// rhythm_piano is not affected
```

## Swing & Humanize

Timing transforms that make patterns feel human.

**Swing** shifts offbeat events (eighth-note positions like 0.5, 1.5, 2.5) later within each beat.

**Humanize** adds random timing jitter.

### Global Settings

Apply to all patterns that don't have their own swing/humanize:

```sc
swing 0.62        // 0.5 = straight, 0.67 = triplet swing
humanize 8        // +/-8ms random jitter per event
```

### Per-Pattern Override

Override global settings for specific patterns (see Patterns section above). This lets you apply different feel to different layers -- swing the hats while keeping the kick on the grid.
