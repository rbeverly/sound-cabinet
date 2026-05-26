# playback Specification

## ADDED Requirements

### Requirement: `play` command streams the score in real time

The `play` subcommand SHALL accept a `.sc` source file and stream the rendered, fully-mastered output through the default audio output device until the score finishes or the user interrupts with Ctrl+C. Playback SHALL be stereo (2-channel) at 44.1 kHz.

#### Scenario: Basic playback
- **WHEN** the user runs `sound-cabinet play <score.sc>`
- **THEN** the engine parses, expands, and begins streaming audio to the default output device
- **AND** stderr prints `Playing... (Ctrl+C to stop)`
- **AND** the program exits cleanly when the score completes or Ctrl+C is pressed

#### Scenario: Missing score path
- **WHEN** the user runs `sound-cabinet play` with no arguments
- **THEN** the program exits non-zero with a usage message naming `<score.sc>`

### Requirement: `-v` / `--verbose` flag prints beat positions and pattern names

The verbose flag SHALL cause the engine to print progress information (beat positions, active pattern names) to stderr during playback.

#### Scenario: Verbose playback
- **WHEN** the user runs `play <score.sc> -v`
- **THEN** stderr includes the suffix `(verbose)` on the `Playing...` line
- **AND** beat positions and pattern names are emitted as playback progresses

### Requirement: `--from <beat>` skips ahead to a specific beat

The `--from <beat>` flag SHALL cause playback to begin at the given beat position rather than beat 0. Pattern expansion still uses the full score timeline; only the playback cursor is advanced. The flag SHALL emit `Skipping to beat <N>...` to stderr.

#### Scenario: Skip to beat 140
- **WHEN** the user runs `play <score.sc> --from 140`
- **THEN** stderr contains `Skipping to beat 140...`
- **AND** audio begins from what would have been beat 140 in the original timeline

#### Scenario: Invalid `--from` value
- **WHEN** `--from` is given a non-numeric value
- **THEN** the program exits non-zero with an error naming `--from requires a number`

### Requirement: `--solo <voices>` plays only the named voices

The `--solo <voices>` flag SHALL accept a comma-separated list of voice names and SHALL mute all voices not in the list. The flag SHALL emit `Solo: <voices>` to stderr at start.

#### Scenario: Solo a single voice
- **WHEN** the user runs `play <score.sc> --solo bass`
- **THEN** stderr contains `Solo: bass`
- **AND** only events labeled `bass` are audible

#### Scenario: Solo multiple voices
- **WHEN** the user runs `play <score.sc> --solo bass,melody`
- **THEN** stderr contains `Solo: bass, melody`
- **AND** only events labeled `bass` or `melody` are audible

### Requirement: `--vu` / `--meters` shows live per-voice VU meters

The `--vu` flag (alias `--meters`) SHALL display real-time per-voice level bars during playback. Voices whose recent RMS is too low SHALL be flagged `(quiet)`; voices whose peak exceeds the clip threshold SHALL be flagged `(clip!)`. Peak hold markers SHALL decay gradually so transient peaks remain visible briefly.

#### Scenario: VU meters during playback
- **WHEN** the user runs `play <score.sc> --vu`
- **THEN** the terminal displays a live, updating per-voice level meter
- **AND** voices with peaks above the clip threshold display the `(clip!)` flag
- **AND** voices with RMS below the quiet threshold display the `(quiet)` flag

### Requirement: `--subfold` enables sub-bass fold-up monitoring (playback only)

The `--subfold` flag SHALL pitch-shift content below ~80 Hz up by one octave and mix it back into playback as a quiet monitoring layer so sub-bass is audible on headphones or small speakers. Fold-up SHALL be playback-only and SHALL never affect WAV output from `render`. The flag SHALL emit `Sub-bass fold-up monitoring active (sub-bass shifted up 1 octave)` to stderr.

#### Scenario: Sub-bass fold-up active
- **WHEN** the user runs `play <score.sc> --subfold`
- **THEN** stderr contains the fold-up activation message
- **AND** sub-bass content below ~80 Hz is also audible at a higher octave in the output stream

#### Scenario: Fold-up does not affect render
- **WHEN** `render` is invoked on the same score
- **THEN** the WAV output does NOT contain the folded-up sub-bass layer

### Requirement: `--env <profile>` mixes environmental noise into playback (playback only)

The `--env <profile>` flag SHALL accept one of `car`, `cafe`, `coffee`, `subway`, or `train` and SHALL mix a calibrated noise profile into the playback stream to test how the mix translates under real-world listening conditions. Aliases `coffee` and `train` resolve to `cafe` and `subway` respectively. The added noise SHALL be playback-only and SHALL never affect rendered WAV output. The flag SHALL emit `Environment simulation: <profile>` to stderr.

#### Scenario: Car noise simulation
- **WHEN** the user runs `play <score.sc> --env car`
- **THEN** stderr contains `Environment simulation: car`
- **AND** a multi-layer noise mix simulating engine rumble, tire noise, wind, and A/C hiss is mixed into the output stream

#### Scenario: Cafe noise simulation
- **WHEN** the user runs `play <score.sc> --env cafe` (or `--env coffee`)
- **THEN** stderr contains `Environment simulation: <profile>`
- **AND** a noise mix simulating room tone, chatter, and clinking is mixed into the output

#### Scenario: Subway noise simulation
- **WHEN** the user runs `play <score.sc> --env subway` (or `--env train`)
- **THEN** stderr contains `Environment simulation: <profile>`
- **AND** a heavy broadband noise mix is mixed into the output

#### Scenario: Unknown environment profile
- **WHEN** the user runs `play <score.sc> --env <unknown>`
- **THEN** stderr contains a warning naming the unknown profile and listing valid options
- **AND** playback proceeds without environmental noise

### Requirement: Realtime A/B master bus bypass toggle

During playback (and piano mode), pressing `m` or `\` SHALL toggle the entire user-defined master bus chain on and off in real time. When bypassed, the dry signal's RMS SHALL be matched to the wet signal's RMS so the comparison is not biased by loudness. The current total gain reduction across master compressor, multiband, and limiter SHALL be displayed as `[ GR -<N> dB ]` in the terminal.

#### Scenario: Toggle master bypass with `m`
- **GIVEN** playback is active with a non-empty user master chain
- **WHEN** the user presses `m` (or `\`)
- **THEN** the user-definable portion of the master chain is bypassed
- **AND** the output is RMS-volume-matched to the previous wet signal
- **AND** the terminal indicates the bypass state

#### Scenario: Gain reduction meter visible
- **WHEN** playback is active
- **THEN** the terminal displays a `[ GR -<N> dB ]` indicator showing the instantaneous combined gain reduction from compressor, multiband, and limiter
