[← Back to README](../README.md)

# Roadmap

This is the development roadmap for Sound Cabinet, roughly in priority order.

## Section enhancements: positioning, ranges, sequence, and scoping

The current section grammar is too rigid -- only `repeat X every N beats` and `play X` (from beat 0). These enhancements make sections a proper arrangement tool.

**Beat range for repeat** -- tile a pattern only within a specific range:

```sc
section intro = 32 beats
  repeat clap_funky until 32
  repeat hats_steady from 8 to 32
  repeat marimba_wonky every 4 beats from 16
```

`from` and `to` are independent keywords: use both, either, or neither. `until` is syntactic sugar for `to`. If `every` is omitted, it defaults to the pattern's own duration. The implicit `every` is resolved during expansion (when pattern durations are known), not at parse time.

**Truncation and overflow** -- patterns that extend beyond the section's declared length truncate at the boundary. Playing a 32-beat pattern inside a 16-beat section gives you just the first half:

```sc
section teaser = 16 beats
  at 0 play full_melody    // full_melody is 32 beats, but only the first 16 play
```

**Implicit section length** -- if the beat count is omitted, the section's duration is computed from its contents:

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

**At-positioning** -- start a pattern at a specific beat within a section:

```sc
section verse = 32 beats
  at 8 play fill_pattern
  at 16 repeat hats every 1 beat
```

**Sequence** -- play patterns one after another (sequential, not simultaneous):

```sc
section verse = 32 beats
  repeat drums every 8 beats
  sequence bass_sparse, bass_active    // 16 + 16 = 32, plays back-to-back
```

`sequence` could evolve to support inline generators:

```sc
sequence (pick [groove_a, groove_b, groove_c] 2 times), breakdown
```

**Repeat blocks inside sections** -- allow `repeat N { pick [...] }` and `repeat N { shuffle [...] }` inside sections:

```sc
section verse = 32 beats
  repeat drums every 8 beats
  repeat 4 {
    pick [groove_a, groove_b]
  }
```

**`with` at all scope levels** -- voice substitution inside sections, sequences, and even inline with specific patterns:

```sc
section evolution = 32 beats
  repeat bass_line from 0 to 16 with {bass = synth_bass}
  repeat bass_line from 16 to 32 with {bass = upright_bass}
```

## Multi-note instrument calls

Allow instruments to accept multiple frequencies, producing a summed chord:

```sc
// Currently required for instrument chords:
at 0 play lead(E5) for 1 beat
at 0 play lead(G5) for 1 beat
at 0 play lead(B5) for 1 beat

// Proposed: multi-note shorthand
at 0 play lead(E5, G5, B5) for 1 beat
```

The engine would instantiate the instrument's signal chain once per frequency, scale each by `1/N`, and sum them.

## Expression ranges

The `->` sweep operator currently only accepts literal numbers (`800 -> 4000`). Expression ranges would allow freq-relative sweeps inside instruments:

```sc
// Currently not supported -- but should be:
instrument pluck = saw(freq) >> lowpass(freq * 8 -> freq * 1.5, 0.6) >> decay(12)
```

This requires extending `Expr::Range` from `Range(f64, f64)` to `Range(Box<Expr>, Box<Expr>)`, and having the graph builder evaluate both sides before constructing the sweep envelope.

## Parallel signal routing

Named internal buses inside `fx` definitions that allow splitting, processing, and recombining signals:

```sc
fx worn_tape = {
  dry: lowpass(1200, 0.3)
  noise: 0.03 * pink() >> highpass(1000, 0.5)
  out: dry + noise
}
```

## Import namespacing

Prevent name collisions when importing multiple kits:

```sc
import voices/lofi-kit.sc as lofi       // lofi.bass, lofi.mel, etc.
import voices/instruments.sc as inst     // inst.rhodes, inst.nylon, etc.
```

## Waveshaping modes

Extend `distort` with named modes beyond tanh soft-clip -- asymmetric clipping (tube warmth), foldback distortion (aggressive harmonics), half-wave rectification (even harmonics):

```sc
saw(C3) >> distort(3.0, "fold")    // foldback
sine(A4) >> distort(2.0, "asym")   // asymmetric / tube-style
```

## Wavetable interpolation modes

Non-linear interpolation modes (cubic, spline) for custom waveforms:

```sc
wave bell cubic = [0.0, 1.0, 1.0, 0.0]    // cubic interpolation
wave harsh = [0.0, 1.0, -1.0, 0.0]         // default: linear
```

## Wave grid syntax

Visual grid definition for waveforms (rows = amplitude, columns = time):

```sc
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

## Multi-cycle waveforms

Compose multiple wave definitions into a longer repeating pattern:

```sc
wave evolving = cycle [wonky, spiky, spiky, wonky]
```

## Tuning & microtonal

Change the reference pitch from the default A4=440 Hz:

```sc
tuning 432        // A4 = 432 Hz -- all notes shift accordingly
```

Beyond alternate reference pitches: support non-12-TET tuning systems (19-TET, 24-TET quarter tones, just intonation, gamelan pelog/slendro). Named scale systems (ragas, maqam, pentatonic modes) could work as selections from a tuning.

## Composable timing transforms

Play-time piping for swing -- apply different swing to different layers in the same section:

```sc
section groove = 16 beats
  repeat hats every 4 beats >> swing 0.7
  repeat kick_pattern every 4 beats
  play bass
```

Also: expressive dynamics (`rush`, `drag`, `push`) for manual performance markup, and algorithmic humanization based on musical structure heuristics.

## Velocity & dynamics

Per-note velocity so drum patterns and melodies feel human instead of mechanical:

```sc
at 0 play kick vel 0.9 for 0.5 beats
at 1 play snare vel 0.6 for 0.25 beats
```

## Sostenuto pedal

Selective sustain -- only holds notes that are already pressed when the pedal goes down. New notes played after are damped normally:

```sc
sostenuto down at 4.0    // captures currently-sounding notes
sostenuto up at 8.0      // releases only those notes
```

## Una corda (soft pedal)

Shifts the piano hammer mechanism to strike fewer strings -- quieter and timbrally darker. In the engine this applies a gain reduction + lowpass filter shift:

```sc
soft down at 4.0
soft up at 8.0
```

## Recording quantize / grid align

Post-processing for piano mode recordings. Snaps raw timestamped note events to the nearest grid division, with swing detection:

```bash
sound-cabinet quantize recorded_1.sc --grid 1/8 -o quantized.sc
sound-cabinet quantize recorded_1.sc --grid 1/8 --detect-swing -o quantized.sc
```

## Standalone gate function

A `gate(duration)` effect that truncates notes to a fixed length, independent of the arpeggiator:

```sc
pad(Cm7) >> gate(0.5) for 4 beats    // quarter-note chops
drone(A2) >> gate(0.125) for 8 beats  // sixteenth-note stutter
```

## Melody pattern discovery

Systematic mining of music theory for reusable melodic contour patterns. Analyze established melodic archetypes from folk, classical, jazz, and pop traditions and encode them as YAML pattern files.

## Metronome mode

A standalone metronome click for practice and recording:

```bash
sound-cabinet metronome 120              // click at 120 BPM
sound-cabinet metronome 120 --time 3/4   // waltz time
```

In piano mode, the metronome activates automatically during recording (F1) and stops when recording stops.

## Algorithmic phrase generation: ornamentation pass

An optional post-processing pass that decorates resolved phrases with mordents, turns, trills, grace notes, and passing tones. Controlled by a density level (0 = clean, 3 = florid/baroque).

See [algorithmic-generation.md](algorithmic-generation.md) for the full design.

## Algorithmic instrument generation

Trait-driven instrument synthesis that builds playable instruments from high-level descriptive vocabulary: `"plucked, decaying, woody, plinky"` resolves to a concrete signal chain through archetype templates and trait-to-parameter mapping.

See [instrument-generation.md](instrument-generation.md) for the full design.

## True peak limiter

The current `BrickwallLimiter` operates on sample values, but the reconstructed analog signal between samples can peak higher. A proper true peak limiter would oversample the detection stage (typically 4x) and apply gain reduction accordingly.

## Format export (MP3, FLAC, AAC)

Direct export to compressed formats for streaming platform upload:

```bash
sound-cabinet render track.sc -o track.mp3 --lufs -14
```

## Automatic equal-loudness compensation

Apply `loudness(freq)` automatically to all instruments via a global `loudness on` directive, removing the need to add it to every instrument definition manually.

## MIDI export (sc2midi)

Render to `.mid` instead of `.wav` so compositions can be brought into a DAW. Combined with the existing `midi2sc.py` importer, this creates a round-trip: MIDI -> .sc -> MIDI.

## Stereo output and panning

Adding stereo would transform the output quality. Requires changing the render pipeline from single-channel to dual-channel buffers and adding a `pan(position)` effect:

```sc
instrument wide_pad = saw(freq) >> lowpass(2000, 0.5) >> pan(0.3)
instrument bass = sine(freq) >> decay(8) >> pan(0.0)   // center
```

## Sample playback

Load audio files and trigger them as voices:

```sc
sample clap = "samples/clap.wav"
sample vocal = "samples/hook.wav"

at 0 play clap for 0.5 beats
at 4 play vocal for 8 beats
```

Would support WAV and potentially FLAC. Pitch-shifting samples by varying playback speed is a natural extension.

## Automation curves

Parameter changes over time beyond the existing linear sweep:

```sc
// Future: named automation curves
curve filter_sweep = [0: 200, 2: 8000, 4: 2000, 8: 200]
sine(0) >> lowpass(filter_sweep, 0.7) for 8 beats
```

Could also support common curve shapes: exponential, logarithmic, S-curve, step.

## Time signature in the DSL

Add `time 3/4` (or `time 6/8`, `time 5/4`, etc.) as a score directive:

```sc
time 3/4
bpm 120
play waltz_pattern
```

## Key signature in the DSL

Add `key Am` or `key D major` as a score directive:

```sc
key A minor
bpm 92
play verse
```

## Transpose

Shift a pattern or entire section up or down by semitones or scale degrees:

```sc
play verse
play verse transpose +5          // up 5 semitones
play chorus transpose -2          // down 2 semitones
play bridge transpose up 3        // up 3 scale degrees (diatonic)
```

## Mixer / levels command

Set relative volumes per voice without editing every `play` line:

```sc
mix bass 0.6, melody 1.0, drums 0.8, pad 0.3
```

## TUI visualization

A terminal UI during playback showing scrolling beat position, active voices, level meters, and waveform.

## Contextual error messages

Replace raw parser errors with actionable human-readable messages that explain what's wrong and how to fix it.

## Score linting and validation

Catch common mistakes before playback: overlapping notes, notes outside range, missing voice definitions, unused patterns:

```bash
sound-cabinet lint song.sc
```

## Count-in for recording

Four metronome clicks before recording starts in piano mode.

## Harmonic analysis

Analyze an existing `.sc` file and report the chord progression, key center, scale usage, and voice ranges:

```bash
sound-cabinet analyze song.sc
```

## Style presets for generation

Predefined combinations of patterns, tempos, and song structures for common genres:

```bash
sound-cabinet generate --style jazz --key Dm --chords "Dm7 G7 Cmaj7 Am7" -o jazz.sc
```

## Counterpoint checker

Flag parallel fifths, parallel octaves, voice crossing, and other voice-leading issues:

```bash
sound-cabinet check-counterpoint song.sc
```

## A/B auditioning

Play two generated variations back-to-back for quick comparison:

```bash
sound-cabinet audition bass_a.sc bass_b.sc
```

## Chorus, flanger, and phaser

Modulated delay effects:

```sc
pad(Cm7) >> chorus(0.5, 0.3)
lead(freq) >> flanger(0.2, 0.7)
keys(freq) >> phaser(0.3, 4)
```

## Tremolo and vibrato

Amplitude modulation (tremolo) and pitch modulation (vibrato):

```sc
violin(freq) >> vibrato(5.0, 0.02)
rhodes(freq) >> tremolo(4.0, 0.3)
```

## Tape stop and speed effects

Playback speed manipulation for transitions and sound design:

```sc
at 32 play melody >> tape_stop(2.0) for 2 beats
```

## Freeze / flatten randomization

Export a deterministic, fully-expanded version of a score with all randomness resolved:

```bash
sound-cabinet freeze song.sc -o frozen-v1.sc --seed 42
```

## Loop recording / overdub

Record a loop, then play it back while recording a second layer on top:

```bash
sound-cabinet piano voices/kit.sc piano --midi --loop 4
```

## Punch-in / punch-out recording

Re-record just a specific beat range of an existing recording:

```bash
sound-cabinet punch recorded_1.sc --from 8 --to 12 --midi
```

## Repeat with variation

Introduce small random changes on each repeat:

```sc
play verse vary 0.2    // 20% variation intensity
```

## Modulation / key change

Shift the key center mid-song:

```sc
key A minor
play verse
play chorus
modulate +2          // up a whole step to B minor
play chorus
```

## Tempo curves

Smooth accelerando and ritardando:

```sc
bpm 80 -> 120 over 16 beats   // accelerando
bpm 120 -> 80 over 8 beats    // ritardando
```

## ADSR envelope generator

Explicit attack-decay-sustain-release envelope:

```sc
pad(freq) >> adsr(0.1, 0.3, 0.6, 0.5) for 4 beats
```

## Ring modulation

Multiply two signals together for metallic and bell-like tones:

```sc
voice(freq) >> ringmod(60)
```

## Noise burst / transient shaper

Tools for designing percussion from scratch:

```sc
noise() >> transient(0.001, 0.05) >> highpass(2000) for 0.25 beats  // hi-hat
noise() >> transient(0.001, 0.3) >> lowpass(200) for 0.5 beats      // kick body
```

## ABC notation export

A simpler alternative to LilyPond for folk music communities:

```bash
sound-cabinet export song.sc -o song.abc --format abc
```

## Import audio for reference

Load a reference track alongside your composition for A/B level comparison:

```bash
sound-cabinet play song.sc --reference reference-track.wav
```

## Undo in watch mode

When a file save introduces a syntax error, automatically fall back to the last working version instead of going silent.

## Bookmark / cue points

Named positions in the score for quick navigation:

```sc
mark "chorus" at 32
mark "bridge" at 64
```

```bash
sound-cabinet play song.sc --from chorus
```

## Project templates

Scaffold a new project directory with voice definitions, empty patterns, and a starter structure:

```bash
sound-cabinet init my-song --template pop
sound-cabinet init my-song --template jazz
sound-cabinet init my-song --template ambient
```

## VST3/AU plugin export

Compile Sound Cabinet instruments and effect chains into native DAW plugins (VST3 for cross-platform, Audio Unit for Logic/GarageBand). The Rust `nih-plug` framework provides the plugin host wrapper.
