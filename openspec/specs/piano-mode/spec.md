# piano-mode Specification

## Purpose
TBD - created by archiving change initial-spec-baseline. Update Purpose after archive.
## Requirements
### Requirement: `piano` command loads a score's definitions and plays an instrument live

The `piano` subcommand SHALL accept a `.sc` source file as its first positional argument and an optional instrument/voice/wave name as its second positional argument. The score's voice/instrument/fx/wave definitions and master bus configuration SHALL be loaded; no playback events are scheduled. With no instrument name given, a default sine + decay tone SHALL be used.

#### Scenario: Load and play an instrument
- **WHEN** the user runs `sound-cabinet piano voices/kit.sc piano`
- **THEN** the engine loads only definitions (voices, instruments, fx, waves, master bus directives, normalize, BPM) from the score
- **AND** key presses trigger the `piano` instrument with the corresponding pitch
- **AND** no events from the score's patterns/sections/play statements are scheduled

#### Scenario: Default tone when no instrument named
- **WHEN** the user runs `sound-cabinet piano voices/kit.sc` (no instrument)
- **THEN** key presses trigger a default sine + decay tone

#### Scenario: Custom wave can be played
- **WHEN** the user runs `sound-cabinet piano examples/wave-test.sc spike`
- **AND** `spike` is a custom `wave` defined in that score
- **THEN** key presses trigger the `spike` waveform at the corresponding pitch

#### Scenario: Missing score path
- **WHEN** the user runs `sound-cabinet piano` with no arguments
- **THEN** the program exits non-zero with a usage message naming `<score.sc> [instrument-name] [--midi [port]] [--velocity <curve>]`

### Requirement: QWERTY keyboard maps to two chromatic octaves C3–C5

The keyboard SHALL map the QWERTY home and top rows to two chromatic octaves spanning C3 to C5, using the GarageBand-style layout:

- Bottom row: `z s x d c v g b h n j m ,` → C3 to C4 (chromatic, including sharps)
- Top row: `q 2 w 3 e r 5 t 6 y 7 u i` → C4 to C5 (chromatic, including sharps)

Other keys SHALL be reserved for piano-mode controls and not mapped to notes.

#### Scenario: Bottom row chromatic C3–C4
- **WHEN** the user presses `z` then `s` then `x`
- **THEN** the notes C3, C#3, D3 sound in succession

#### Scenario: Top row chromatic C4–C5
- **WHEN** the user presses `q` then `2` then `w`
- **THEN** the notes C4, C#4, D4 sound in succession

#### Scenario: Key release ends the note
- **WHEN** a key is released
- **THEN** the corresponding note transitions to its release/decay phase (unless the sustain pedal is down)

### Requirement: MIDI keyboard support with auto-detect

The `--midi` flag SHALL enable MIDI input. With no port index, the first available MIDI input device SHALL be auto-detected and connected. With a numeric port index (`--midi <N>`), the device at that index SHALL be used. MIDI and the QWERTY keyboard SHALL work simultaneously when MIDI is enabled.

#### Scenario: Auto-detect MIDI
- **WHEN** the user runs `piano voices/kit.sc piano --midi`
- **THEN** the program connects to the first available MIDI input device
- **AND** both the QWERTY keyboard and the MIDI keyboard can trigger notes

#### Scenario: Specific MIDI port
- **WHEN** the user runs `piano voices/kit.sc piano --midi 0`
- **THEN** the program connects to MIDI input device at index 0

### Requirement: `--velocity <curve>` selects a MIDI velocity-to-gain curve

The `--velocity` flag SHALL accept one of `linear`, `soft`, `supersoft` (aliases `super-soft`, `super_soft`), `hard`, or `full`. The selected curve SHALL map raw MIDI velocity (1–127) to gain (0.0–1.0) as follows, where `v = raw / 127`:

| Curve | Mapping |
|---|---|
| `linear` | `v` (1:1, default) |
| `soft` | `v ^ 0.5` (square root — boosts quiet) |
| `supersoft` | `v ^ 0.25` (fourth root — strong boost) |
| `hard` | `v ^ 2.0` (square — suppresses quiet) |
| `full` | always 1.0 (ignores velocity) |

Without `--velocity`, the curve SHALL default to `linear`.

#### Scenario: Default velocity curve is linear
- **WHEN** the user runs `piano voices/kit.sc piano --midi`
- **THEN** MIDI velocity maps linearly to gain (`raw / 127`)

#### Scenario: Supersoft curve
- **WHEN** the user runs `piano voices/kit.sc piano --midi --velocity supersoft`
- **THEN** MIDI velocity 32 maps to gain `(32/127)^0.25 ≈ 0.71` rather than `0.25`

#### Scenario: Full velocity
- **WHEN** `--velocity full` is given
- **THEN** every note plays at gain 1.0 regardless of raw velocity

#### Scenario: Unknown velocity curve
- **WHEN** the user supplies a value not in the listed set
- **THEN** the program exits non-zero with `Unknown velocity curve '<value>'. Options: linear, soft, supersoft, hard, full`

### Requirement: Sustain pedal (F4 keyboard, MIDI CC64) extends note tails

The keyboard's `F4` key SHALL toggle the sustain pedal on and off. Independently, MIDI Control Change #64 (the standard sustain pedal CC) SHALL be recognized: CC64 ≥ 64 = pedal down, CC64 < 64 = pedal up. When the pedal is down, notes that would otherwise be released SHALL continue ringing until the pedal comes up.

#### Scenario: F4 toggles sustain
- **WHEN** the user presses F4 with no pedal currently active
- **THEN** subsequent notes ring after their key is released
- **WHEN** the user presses F4 again
- **THEN** all sustained notes are released

#### Scenario: MIDI sustain pedal
- **WHEN** the connected MIDI keyboard sends a CC64 value ≥ 64
- **THEN** the sustain pedal is engaged for all subsequent note-offs
- **WHEN** CC64 drops below 64
- **THEN** all sustained notes are released

### Requirement: Recording controls (F1/F2/F3) capture and save played notes

The function keys SHALL implement recording controls:

- `F1`: start/stop recording. When recording starts, a metronome click SHALL sound on every beat at the engine's current BPM.
- `F2`: save the current recording to a file named `recorded_<N>.sc` in the current directory, where `<N>` is the next available integer not colliding with existing files.
- `F3`: discard the current recording.
- `Esc`: quit piano mode.

The saved file SHALL contain:
- An `import` statement referencing the original voice file
- A `bpm` directive matching the recording tempo
- An `at <beat> play <instrument>(<note>) for <M> beats` line for each recorded note, with beat positions relative to the recording start
- `pedal down`/`pedal up` lines for any sustain pedal events captured during the recording

#### Scenario: Start recording with metronome
- **WHEN** the user presses F1 with no active recording
- **THEN** recording starts
- **AND** a metronome click is audible on every beat

#### Scenario: Save recording
- **WHEN** the user presses F2 with an active recording
- **THEN** the recording is written to `recorded_<N>.sc` where `<N>` does not collide with existing files
- **AND** the file contains an `import` of the original voice file, a `bpm` directive, the played notes as `at` events, and any pedal events

#### Scenario: Discard recording
- **WHEN** the user presses F3
- **THEN** the current recording buffer is cleared
- **AND** no file is written

#### Scenario: Quit piano mode
- **WHEN** the user presses `Esc`
- **THEN** the audio stream is shut down
- **AND** the program exits with status 0

### Requirement: Recorded `.sc` file is directly importable

The output of an F2 save SHALL be valid `.sc` syntax that, when imported into another score via `import recorded_<N>.sc` and played (e.g. via `play recorded_pattern`), reproduces the original performance with the same instrument, tempo, and timing.

#### Scenario: Recorded file imports and plays back
- **GIVEN** a recording was saved as `recorded_1.sc` from piano mode using instrument `piano` at 120 BPM
- **WHEN** another score contains `import recorded_1.sc` followed by an invocation of the recorded events
- **THEN** the imported events use the `piano` instrument from the original voice file
- **AND** the playback tempo matches the recording's 120 BPM
- **AND** each note's beat offset and duration matches what was played live

