# algorithmic-generation Specification

## ADDED Requirements

### Requirement: `generate` command consumes a YAML pattern file and produces `.sc` variations

The `generate` subcommand SHALL require these flags:

- `--pattern <file.yaml>` — path to a YAML pattern, drum-pattern, or song file
- `--key <note>` — root pitch class (e.g. `D`, `Bb`)
- `--mode <mode>` — diatonic mode (e.g. `major`, `minor`, `dorian`, `mixolydian`)
- `--chords "<progression>"` — space-separated chord progression (e.g. `"Dm7 G7 Cmaj7"`)
- `--voice <name>` — name of the Sound Cabinet voice to play the generated notes through

It SHALL accept these optional flags:

- `--range <low-high>` — pitch range bound (e.g. `C2-G3`); when omitted, a sensible default by pattern type is used
- `--variations <N>` — number of variations to generate (default: 5)
- `--seed <N>` — RNG seed for deterministic output (default: random)
- `-o <output.sc>` — output path; omit to print to stdout

The output SHALL be valid `.sc` source containing one named `pattern` per variation, each scoped to the parsed pattern's total beat length.

#### Scenario: Generate five bass variations to file
- **WHEN** the user runs `sound-cabinet generate --pattern patterns/bass/walking-jazz.yaml --key D --mode dorian --chords "Dm7 G7 Cmaj7 Am7" --voice bass --variations 5 --seed 42 -o out.sc`
- **THEN** the file `out.sc` is written containing 5 named `pattern` definitions
- **AND** each pattern represents the source pattern resolved against the chord progression
- **AND** stderr contains `Generated 5 variations -> out.sc`
- **AND** running the command again with the same `--seed 42` produces byte-identical output

#### Scenario: Print to stdout when `-o` omitted
- **WHEN** the user runs `generate ... ` without `-o`
- **THEN** the generated `.sc` source is printed to stdout
- **AND** stderr does NOT contain a `Generated ... -> ...` line

#### Scenario: Missing required flag
- **WHEN** the user omits any of `--pattern`, `--key`, `--mode`, `--chords`, or `--voice`
- **THEN** the program exits non-zero with an error naming the missing flag and an example usage

#### Scenario: Output directory is created if missing
- **WHEN** the user specifies `-o some/missing/dir/out.sc` and `some/missing/dir/` does not yet exist
- **THEN** the parent directories are created before writing the file

### Requirement: Pattern files support rhythm + contour + emphasis vocabulary

A pattern file SHALL be a YAML document with the following structure:

- `name`: string — human-readable name (used in comments)
- `type`: string — one of `bass`, `melody`, `accomp`/`accompaniment`, or others (controls default range)
- `tags`: list of strings (optional metadata)
- `time`: time signature (e.g. `4/4`, `3/4`, or `any`)
- `rhythm`: object with `hits:` — a list of note-length tokens (`1/4`, `1/8`, `1/16`, `1/8.` for dotted-eighth, `1/4+1/8` for tied notes across bar lines, `~` for a rest, `remainder` for "fill the host note")
- `contour`: list of contour tokens (vocabulary below)
- `emphasis`: list of dynamics tokens — `strong` (gain 1.0), `medium` (0.7), `weak` (0.4), `ghost` (0.2)
- `notes`: free-text description (optional)

Contour vocabulary SHALL include:

| Token | Meaning |
|---|---|
| `root` | Scale degree 1 (or chord root, context-dependent) |
| `hold` | Repeat previous pitch |
| `step_up` / `step_down` | Move one diatonic step |
| `half_up` / `half_down` | Move one chromatic semitone |
| `leap_up_N` / `leap_down_N` | Jump N diatonic steps |
| `chord_low` / `chord_mid` / `chord_high` | Chord tones, ordered by pitch |
| `approach` | Chromatic half-step into next bar's target |
| `neighbor_up` / `neighbor_down` | Step away and return (ornamental) |
| `passing` | Diatonic step connecting two chord tones |

#### Scenario: Walking jazz bass pattern resolves
- **GIVEN** a pattern file with rhythm `[1/4, 1/4, 1/4, 1/4]`, contour `[root, step_up, step_up, approach]`, emphasis `[strong, weak, weak, medium]`, and `type: bass`
- **AND** invoked with `--key D --mode dorian --chords "Dm7 G7 Cmaj7 Am7"`
- **THEN** each variation contains one bar (4 beats) of quarter-note events
- **AND** beat 1 is the chord root, beat 2/3 are diatonic step-up moves, beat 4 is a chromatic approach to the next bar's root

### Requirement: Default pitch range derived from pattern `type`

When `--range` is omitted, the generator SHALL apply a default range based on the pattern's `type` field:

- `bass` → C2 to G3
- `melody` → C4 to C6
- `accomp` or `accompaniment` → C3 to C5
- any other type → C3 to C5

#### Scenario: Default bass range
- **WHEN** a pattern with `type: bass` is generated without `--range`
- **THEN** all generated notes fall within C2–G3 (inverting intervals when needed to stay in range)

#### Scenario: Default melody range
- **WHEN** a pattern with `type: melody` is generated without `--range`
- **THEN** all generated notes fall within C4–C6

### Requirement: Drum pattern files generate per-voice drum patterns

When the pattern file's top-level shape contains a `voices` key (a drum pattern), the generator SHALL treat it as a drum pattern: each named voice has its own `rhythm:` and the output contains one or more named drum-pattern variations spanning the longest voice's beat length.

#### Scenario: Drum pattern produces multi-voice variations
- **GIVEN** a YAML file with `voices: [kick: {...}, snare: {...}, hat: {...}]`
- **WHEN** the user runs `generate --pattern <file> ... --variations 3`
- **THEN** the output contains 3 named drum-pattern variations
- **AND** each variation contains `at <beat> play kick/snare/hat for <M> beats` events
- **AND** stderr contains `Generated 3 drum variations -> <path>` when `-o` is set

### Requirement: Song files generate full multi-part song scaffolds

When the pattern file's top-level shape contains a `parts` key, the generator SHALL treat it as a song file: a structured composition with multiple labeled parts. Each part's pattern is resolved and emitted to the output `.sc` source.

#### Scenario: Song file dispatches to song generator
- **GIVEN** a YAML file with a top-level `parts` key
- **WHEN** the user runs `generate --pattern <file> ...`
- **THEN** the song-mode generator runs and emits the full song's parts as `.sc`

### Requirement: Generated output is valid `.sc` source

The generator SHALL produce `.sc` source that:
- Begins with a header comment identifying the source pattern, key, mode, and chord progression
- Defines one `pattern <name> = <N> beats` per variation, with deterministic names (e.g. `bas_a`, `bas_b`, ...) suitable for direct reference via `play <name>` or `pick [<name>, ...]`
- Resolves all contour tokens to concrete note-name `at <beat> play <voice>(<note>) for <duration> beats` events

#### Scenario: Output header includes generation parameters
- **WHEN** generation completes
- **THEN** the output begins with a comment line documenting the source pattern, key, mode, and chord progression

#### Scenario: Generated patterns are `pick`-compatible
- **WHEN** multiple variations are generated with the same key, mode, and chord progression
- **THEN** all variations share the same total beat length
- **AND** all variations can be combined in a `pick [<name1>, <name2>, ...]` block without timing or harmonic conflicts

### Requirement: Mode and chord parsing accept standard music theory names

The `--mode` flag SHALL accept any of the standard diatonic mode names: `major`, `minor`, `dorian`, `phrygian`, `lydian`, `mixolydian`, `aeolian`, `locrian` (and minor as an alias for aeolian where natural).

The `--chords` value SHALL be parsed as space-separated chord names matching the form `Root[Accidental][:Quality]` (e.g. `Dm7`, `G7`, `Cmaj7`, `Am7`, `Bb`). Unknown modes or malformed chords SHALL produce a parse error with a clear message.

#### Scenario: Invalid mode
- **WHEN** the user runs `generate ... --mode bogus`
- **THEN** the program exits non-zero with an error naming the unknown mode

#### Scenario: Empty chord progression
- **WHEN** `--chords ""` resolves to no chords
- **THEN** the program exits non-zero with `At least one chord is required`
