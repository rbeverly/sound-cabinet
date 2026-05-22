# dsl-syntax Specification

## Purpose
Define the syntax and structural semantics of the Sound Cabinet score language (`.sc` files): top-level directives, declarations (`voice`, `instrument`, `fx`, `wave`), composition primitives (`pattern`, `section`, `play`, `repeat`, `sequence`, `sample`, `pick`, `shuffle`), tempo/swing/humanize controls, sustain pedal, voice substitution, imports, and the lexical structure (note names, chord names, comments, identifiers, numbers).

This spec defines the *language surface* — what syntax is accepted and how its constructs compose. The audible result of an expression's signal chain is defined in the [[audio-engine]] spec; the always-on master bus chain is defined in [[master-bus]].

## Requirements

### Requirement: Lexical structure

A `.sc` file SHALL be UTF-8 text composed of statements separated by newlines. Lines beginning with `//` (after optional whitespace) SHALL be comments and ignored by the parser. Whitespace within a statement SHALL be a space or tab; multiple whitespace characters SHALL be equivalent to one. Blank lines SHALL be permitted and ignored.

Identifiers SHALL start with an ASCII letter and may contain letters, digits, and underscores. Numbers SHALL be optionally-signed decimal numerals (`-?[0-9]+(\.[0-9]+)?`). String values are not first-class in the score language; identifiers are used in their place.

#### Scenario: Comment lines ignored
- **WHEN** a line begins with `//` (after whitespace)
- **THEN** the parser discards the line

#### Scenario: Blank lines permitted
- **WHEN** a blank line appears between statements
- **THEN** the parser proceeds to the next statement

#### Scenario: Tabs and spaces interchangeable
- **WHEN** indentation or token separation uses tabs OR spaces
- **THEN** the parser treats them equivalently

### Requirement: Note names

Note names SHALL be of the form `Letter[Accidental]Octave`:

- `Letter` is one of `A`–`G`
- `Accidental` is optional and is one of `#`, `s` (sharp), or `b` (flat)
- `Octave` is a single digit `0`–`9`

Note names SHALL be valid wherever a frequency is expected (oscillator arguments, arp notes, etc.). `A4` corresponds to 440 Hz, `C4` to middle C (~261.63 Hz). Either `s` or `#` MAY be used for sharps; both SHALL produce the same pitch.

#### Scenario: Standard note names
- **WHEN** `sine(A4)` appears in an expression
- **THEN** the oscillator runs at 440 Hz

#### Scenario: Sharp with `#` or `s`
- **WHEN** the user writes `Fs4` or `F#4`
- **THEN** both parse to F-sharp 4 and produce the same frequency

### Requirement: Chord names

Chord names SHALL be of the form `Root[Accidental][Octave]:Quality`. The colon separates the root from the quality. Without an explicit octave, the chord root defaults to octave 4. The accepted qualities SHALL include (longest match first to avoid ambiguity):

`mmaj7`, `m7b5`, `maj9`, `min9`, `add9`, `maj7`, `min7`, `dim7`, `aug7`, `dom9`, `dom7`, `sus2`, `sus4`, `min`, `maj`, `dim`, `aug`, `m9`, `m7`, `m6`, `m`, `9`, `7`, `6`.

#### Scenario: Chord with default octave
- **WHEN** the user writes `C:m7`
- **THEN** the chord resolves to C4 minor 7th

#### Scenario: Chord with explicit octave
- **WHEN** the user writes `Ab3:maj7`
- **THEN** the chord resolves to A-flat major 7th at octave 3

### Requirement: `bpm` sets tempo at the current cursor position

The directive `bpm <N>` SHALL set the playback tempo to `N` beats per minute. When `bpm` appears mid-score (after some `play` statements), it SHALL only apply to events scheduled after that point — earlier events keep the previous tempo.

#### Scenario: Single bpm at top
- **WHEN** a score begins with `bpm 120` followed by play statements
- **THEN** all events use 120 BPM

#### Scenario: Tempo change mid-score
- **WHEN** a score contains `play intro`, then `bpm 82`, then `play chorus`
- **THEN** `intro` plays at the previous tempo and `chorus` plays at 82 BPM

### Requirement: `swing` and `humanize` directives apply globally

`swing <N>` SHALL set a global swing amount where 0.5 means straight, ~0.67 means triplet swing. Swing SHALL shift offbeat events (half-beat positions) later within each beat by the specified amount.

`humanize <ms>` SHALL set a global timing-jitter amount in milliseconds. Each scheduled event's start time SHALL be offset by a random ±`<ms>` value.

These directives SHALL apply to all patterns that do not override them with per-pattern modifiers (see [pattern modifiers requirement](#requirement-pattern-and-section-definitions)).

#### Scenario: Apply global swing
- **WHEN** the score contains `swing 0.62`
- **THEN** all offbeat eighth-note positions are shifted to 62% of the beat in patterns that don't override the global value

#### Scenario: Apply global humanize
- **WHEN** the score contains `humanize 8`
- **THEN** each event's start time is jittered by up to ±8 ms

### Requirement: `voice`, `fx`, `instrument`, and `wave` declarations

Each declaration kind SHALL be a single line introducing a name bound to a definition:

- `voice <name> = <expr>` — A complete signal graph; played by name with `play <name>` (no frequency parameter).
- `fx <name> = <expr>` — A named, reusable chain of effects with no source; inserted into another chain via `>>`.
- `instrument <name> = <expr>` — A signal graph template that uses `freq` as a variable; `freq` SHALL be substituted with the actual frequency in Hz when the instrument is invoked as `name(<freq_or_note>)`.
- `wave <name> = [<n1>, <n2>, ...]` — A custom periodic waveform defined as a list of sample points (one cycle); used as a function call `name(<freq_or_note>)` in expressions.

Names SHALL be unique within their declaration kind. Redeclaration SHALL replace the previous definition (last definition wins). The order of declarations relative to use is not significant — definitions may appear before or after their references, but ALL definitions must be reachable from imports before the first use that needs them (lazy resolution at expansion).

#### Scenario: Voice declaration
- **WHEN** the score contains `voice pad = (saw(C3) + 0.5 * sine(C4)) >> lowpass(2000, 0.7) >> reverb(0.6, 0.4, 0.3)`
- **THEN** the name `pad` resolves to that complete signal graph
- **AND** `at 0 play pad for 4 beats` plays it

#### Scenario: Instrument with `freq` substitution
- **WHEN** the score contains `instrument piano = saw(freq) >> lowpass(freq * 4, 0.7) >> decay(8)`
- **AND** the user writes `at 0 play piano(C4) for 2 beats`
- **THEN** `freq` is substituted with the Hz value of `C4` (~261.63) everywhere in the expression
- **AND** the filter cutoff becomes `261.63 * 4`

#### Scenario: Multi-note instrument call
- **WHEN** the user writes `at 0 play piano(C4, E4, G4) for 2 beats`
- **THEN** the instrument's signal chain is instantiated once per frequency
- **AND** each instance is scaled by `1/N` (where N = 3)
- **AND** the three instances are summed

#### Scenario: Custom wave definition and use
- **WHEN** the score contains `wave plateau = [0.0, 0.4, 0.8, 1.0, ...]`
- **AND** the user writes `plateau(C3)`
- **THEN** the array is interpreted as one cycle and read at the rate corresponding to C3's frequency
- **AND** linear interpolation is used between sample points

#### Scenario: `fx` chain has no source
- **WHEN** the score contains `fx hall = reverb(0.8, 0.4, 0.35) >> delay(0.3, 0.2, 0.15)`
- **AND** the user writes `saw(C3) >> hall`
- **THEN** the saw output flows through the `hall` chain's reverb then delay

### Requirement: `at <beat> play <expr> for <N> beats` schedules events at the top level

The top-level scheduling statement SHALL be `at <beat> play <expr> for <N> beat[s]`. `<beat>` is the absolute beat position from the start of the score (or the start of the section/pattern when nested). `<expr>` is any expression — a voice/instrument name, a signal graph, or a chord/arp invocation. `<N>` is the duration in beats. Either `beat` or `beats` SHALL be accepted.

#### Scenario: Schedule an event at beat 0
- **WHEN** the score contains `at 0 play sine(A4) for 2 beats`
- **THEN** a sine wave at 440 Hz plays from beat 0 to beat 2

#### Scenario: Schedule with a named voice
- **WHEN** the score contains `at 4 play pad for 8 beats`
- **THEN** the previously-declared `pad` voice plays from beat 4 to beat 12

#### Scenario: Singular `beat`
- **WHEN** the score contains `at 0 play kick for 1 beat`
- **THEN** the parser accepts `beat` as equivalent to `beats`

### Requirement: `pattern` and `section` definitions

`pattern <name> = <N> beat[s] [swing <S>] [humanize <H>]` SHALL define a reusable group of events with a duration of `<N>` beats. Pattern modifiers `swing` and `humanize` SHALL override the global values within the pattern. Beat offsets inside a pattern SHALL be relative to wherever the pattern is played.

A pattern body SHALL consist of one or more `at <beat> play <expr> for <M> beats` lines (top-level event syntax). Patterns SHALL NOT contain nested patterns or sections.

`section <name> = <N> beat[s] [with {<mappings>}]` SHALL define a reusable section composing patterns and inline events over a duration. The `= <N> beats` clause MAY be omitted, in which case the section's length SHALL be inferred from the latest endpoint of its contents. `= 0 beats` SHALL also signal inferred length.

A section body SHALL accept these entry types:
- `repeat <pattern_ref> [every <N> beat[s]] [from <N>] [to|until <N>] [with {<mappings>}] [gain <amount>]` — tile the pattern at intervals
- `play <pattern_ref> [from <N>] [with {<mappings>}] [gain <amount>]` — play once
- `at <beat> play <pattern_ref> [with {...}] [gain <amount>]` — play at a specific beat
- `at <beat> repeat <pattern_ref> [every ...] [...] [with {...}] [gain <amount>]` — repeat from a beat
- `sequence <ref1>, <ref2>, ... [with {...}] [gain <amount>]` — play sequentially, back-to-back
- `at <beat> play <expr> for <N> beats` — inline one-off event
- `repeat <N> { pick [<items>] | shuffle [<items>] }` — repeat block with random selection

#### Scenario: Pattern with relative beat offsets
- **WHEN** the score contains:
  ```
  pattern drums = 4 beats
    at 0 play kick for 0.5 beats
    at 2 play snare for 0.25 beats
  ```
- **AND** `repeat drums every 4 beats from 8 to 16` appears in a section
- **THEN** at section beats 8, 12, `kick` plays at relative beat 0 and `snare` at relative beat 2

#### Scenario: Per-pattern swing override
- **WHEN** the score contains `pattern hats = 4 beats swing 0.65 humanize 5`
- **THEN** events in `hats` use swing 0.65 and humanize 5 ms regardless of any global `swing`/`humanize` values

#### Scenario: Section with inferred length
- **WHEN** the score contains:
  ```
  section auto
    at 0 play intro_8beat
    at 8 play verse_32beat
  ```
- **THEN** the section's total length is 40 beats (max endpoint)

#### Scenario: Section length truncation
- **WHEN** the score contains:
  ```
  section teaser = 16 beats
    at 0 play full_melody
  ```
- **AND** `full_melody` is 32 beats long
- **THEN** only the first 16 beats of `full_melody` play; events past beat 16 are discarded

#### Scenario: Patterns cannot nest
- **WHEN** a pattern body contains another `pattern` or `section` declaration
- **THEN** the parser emits an error with a hint to define patterns separately and reference them by name

### Requirement: `repeat` modifiers (`every`, `from`, `to`/`until`)

In a section entry of the form `repeat <pattern_ref>`, the modifiers SHALL combine independently:

- `every <N> beats` — tile at intervals of `<N>` beats. When omitted, SHALL default to the pattern's own duration.
- `from <N>` — start tiling at section beat `<N>` (inclusive)
- `to <N>` or `until <N>` — stop tiling at section beat `<N>` (exclusive). `until` is sugar for `to`.

The implicit `every` is resolved during expansion (after pattern durations are known), not at parse time.

#### Scenario: All three modifiers combined
- **WHEN** the section contains `repeat hats every 1 beat from 8 to 32`
- **THEN** `hats` is tiled at 1-beat intervals starting at section beat 8, ending at section beat 32

#### Scenario: Implicit `every`
- **WHEN** the section contains `repeat hats_steady from 8 to 32` and `hats_steady` has duration 1 beat
- **THEN** `every` defaults to 1 beat and tiling occurs every 1 beat from 8 to 32

#### Scenario: `until` synonym
- **WHEN** the section contains `repeat clap until 16`
- **THEN** tiling begins at section beat 0 (default `from`) and stops at section beat 16

### Requirement: `sequence` plays pattern references back-to-back

`sequence <ref1>, <ref2>, ...` (in a section) SHALL play each reference in turn, with each starting immediately after the previous one finishes. The total duration of the sequence SHALL equal the sum of the referenced patterns' durations.

#### Scenario: Two-pattern sequence
- **WHEN** the section contains `sequence bass_a, bass_b` and both patterns are 16 beats
- **THEN** `bass_a` plays from beat 0 to 16, then `bass_b` plays from beat 16 to 32

### Requirement: `sample(<pattern>, <start>[, <end>])` extracts a sub-pattern by beat range

`sample(<pattern_name>, <start>[, <end>])` SHALL produce an anonymous sub-pattern whose events are the original pattern's events filtered to start beats ≥ `<start>` and < `<end>` (when `<end>` is provided), with beat offsets rebased to start at 0. When `<end>` is omitted, the slice extends to the original pattern's end. `sample(...)` SHALL be valid anywhere a pattern reference is valid: top-level `play`, section `play`/`repeat`/`sequence`, etc.

#### Scenario: Slice the first 16 beats
- **WHEN** the user writes `play sample(melody, 0, 16)`
- **THEN** only events from `melody` with start beat ≥ 0 and < 16 are played
- **AND** beat offsets are preserved (rebased starting at 0)

#### Scenario: Slice from a beat to the end
- **WHEN** the user writes `play sample(melody, 16)`
- **THEN** only events with start beat ≥ 16 are played, with beat offsets rebased to 0

#### Scenario: Slice in a sequence
- **WHEN** the user writes `sequence sample(melody, 0, 16), sample(melody, 16, 32)`
- **THEN** the two slices play back-to-back

### Requirement: Top-level `play`, `repeat`, `sequence`, `pick`, `shuffle`

At the top level (outside any pattern or section), the following constructs SHALL be available:

- `play <pattern_ref> [gain <amount_or_curve>] [fade in <N> beats] [fade out <N> beats]` — play sequentially after the previous top-level statement
- `repeat <N> { pick [<items>] }` or `repeat <N> { shuffle [<items>] }` — repeat block with random selection
- `bpm <N>` — tempo change at the current cursor

Top-level `play` statements SHALL advance an implicit cursor; each `play` starts after the previous one finishes.

`pick [<a>, <b>, ...]` SHALL select one item at random per iteration. Items MAY be weighted as `<name>:<weight>` (default weight 1). `shuffle [<a>, <b>, ...]` SHALL play all items in a random order each iteration.

#### Scenario: Sequential top-level play
- **WHEN** the score contains `play intro`, then `play verse`, then `play chorus`
- **THEN** each section plays after the previous one finishes (no explicit beats needed)

#### Scenario: Repeat with weighted pick
- **WHEN** the score contains `repeat 8 { pick [verse_a:2, verse_b:2, chorus:1] }`
- **THEN** 8 iterations occur, each selecting one item; weights cause `verse_a` and `verse_b` to be selected about twice as often as `chorus`

#### Scenario: Top-level fade
- **WHEN** the score contains `play intro fade in 4 beats fade out 4 beats`
- **THEN** `intro` fades in linearly over its first 4 beats and fades out linearly over its last 4 beats

#### Scenario: Top-level gain automation
- **WHEN** the score contains `play verse gain 0.5`
- **THEN** the gain envelope of `verse` is scaled by 0.5 throughout

### Requirement: `with` voice substitution (three scopes)

The `with` clause SHALL substitute voice names at play time, with three scope levels (innermost wins):

1. **Global `with <name1> = <name2>, ...`** (a top-level statement) — applies to all subsequent events
2. **Section-level `section <name> = <N> beats with {<mappings>}`** — overrides globals for that section
3. **Per-entry `... with {<mappings>}`** on a section entry — overrides for that one entry

#### Scenario: Global substitution
- **WHEN** the score contains `with kick = analog_kick, snare = tight_snare`
- **AND** later `repeat drums every 4 beats` is executed where `drums` references `kick` and `snare`
- **THEN** all references to `kick` and `snare` resolve to `analog_kick` and `tight_snare` respectively

#### Scenario: Per-entry substitution
- **WHEN** a section entry is `repeat drums every 4 beats with {hat = shaker}`
- **THEN** within that one repeat, `hat` references resolve to `shaker`, but other entries in the section continue to use the global/section bindings

### Requirement: `pedal down` and `pedal up` extend note tails

`pedal down at <beat>` SHALL begin sustaining notes (extending their duration to the pedal-up point) from beat `<beat>`. `pedal up at <beat>` SHALL release sustained notes. The default scope SHALL be all voices.

A specific voice or list of voices MAY be given:

- `pedal down <voice> at <beat>` — sustain only that voice
- `pedal down [<v1>, <v2>, ...] at <beat>` — sustain the listed voices

The voice name MAY be a `with`-substituted alias; pedal scope SHALL respect substitution.

#### Scenario: Global pedal
- **WHEN** the score contains `pedal down at 4.0` and `pedal up at 8.0`
- **THEN** between beats 4 and 8, any note whose nominal duration ends is extended to beat 8

#### Scenario: Voice-scoped pedal
- **WHEN** the score contains `pedal down piano at 4.0` and `pedal up piano at 8.0`
- **THEN** only `piano` notes are sustained; other voices play their normal durations

#### Scenario: Multi-voice pedal
- **WHEN** the score contains `pedal down [piano, strings] at 4.0`
- **THEN** both `piano` and `strings` notes are sustained until their corresponding `pedal up`

### Requirement: `normalize <name> <target>` levels an instrument or voice

`normalize <name> <target>` SHALL set a target output level for the named instrument or voice. The target SHALL be on a 0.0–1.0 scale where 1.0 = full scale and 0.5 = -6 dB. The engine SHALL render short test tones across multiple frequencies through the named source, measure average RMS, and apply a gain correction so the source's output reaches the target.

#### Scenario: Normalize an instrument
- **WHEN** the score contains `normalize bass 0.5`
- **THEN** the engine measures `bass`'s typical RMS and applies a gain so its output averages -6 dB

### Requirement: `import <path>` includes another score file

`import <path>` SHALL parse and inline another `.sc` file's contents at the point of the import. The path SHALL be resolved relative to the importing file's directory. All declarations (voice/instrument/fx/wave/patterns/sections), directives, and statements from the imported file SHALL become part of the importing file's namespace and timeline. Circular imports SHALL produce a clear parse error.

#### Scenario: Import a voice kit
- **WHEN** the score contains `import voices/kit.sc`
- **AND** `voices/kit.sc` defines `instrument piano = ...`
- **THEN** `piano` is available for use in the importing score

#### Scenario: Relative path resolution
- **WHEN** `examples/song.sc` contains `import voices/kit.sc`
- **THEN** the path resolves to `examples/voices/kit.sc`

### Requirement: Expression operator precedence

Expressions SHALL follow operator precedence (highest first):

1. Function call `name(args...)` and parentheses `(<expr>)`
2. Multiplication `*` and division `/`
3. Addition `+` and subtraction `-`
4. Pipe chain `>>`

The range operator `<a> -> <b>` (parameter sweep) SHALL be valid only as a function argument and SHALL bind tighter than any other operator within that argument position.

#### Scenario: Pipe is lowest precedence
- **WHEN** the user writes `0.5 * saw(C3) >> lowpass(800, 0.7)`
- **THEN** the expression parses as `(0.5 * saw(C3)) >> lowpass(800, 0.7)` — the multiplication binds before the pipe

#### Scenario: Parentheses force grouping
- **WHEN** the user writes `(saw(C3) + sine(C4)) >> lowpass(1000, 1.0)`
- **THEN** the sum is computed before being fed into the lowpass

### Requirement: Reserved keywords cannot be used as identifiers

The following SHALL be reserved keywords and SHALL NOT be valid as voice/instrument/fx/wave names: `voice`, `fx`, `instrument`, `wave`, `pattern`, `section`, `bpm`, `swing`, `humanize`, `with`, `play`, `at`, `for`, `beat`, `beats`, `from`, `to`, `until`, `every`, `repeat`, `sequence`, `sample`, `pick`, `shuffle`, `pedal`, `down`, `up`, `master`, `chain`, `compress`, `expand`, `saturate`, `multiband`, `curve`, `excite`, `ceiling`, `gain`, `low`, `mid`, `high`, `fade`, `in`, `out`, `normalize`, `import`, `freq`.

The parser SHALL warn when a declared name collides with a built-in function name (e.g. `sine`, `saw`, `lowpass`, `reverb`) so the user can recognize the shadowing.

#### Scenario: Reserved keyword as identifier
- **WHEN** the user writes `voice with = ...`
- **THEN** the parser emits an error naming `with` as a reserved keyword

#### Scenario: Built-in name shadowing warning
- **WHEN** the user writes `voice sine = ...`
- **THEN** the parser emits a warning that `sine` shadows a built-in function
