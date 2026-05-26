# rendering Specification

## ADDED Requirements

### Requirement: Render command writes stereo WAV at 44.1 kHz

The `render` subcommand SHALL accept a `.sc` source file and write a stereo (2-channel) 16-bit PCM WAV file at 44.1 kHz sample rate to the path given by `-o`. The render SHALL traverse the full expanded score from beat 0 to the last scheduled event and apply the score's master bus chain to the output.

#### Scenario: Basic render to WAV
- **WHEN** the user runs `sound-cabinet render <score.sc> -o <output.wav>`
- **THEN** the engine parses and expands the score
- **AND** the engine renders the full score through the master bus chain
- **AND** a 2-channel 16-bit PCM WAV file at 44.1 kHz is written to `<output.wav>`
- **AND** the program exits with status 0 and prints `Rendered to <output.wav>` on stderr

#### Scenario: Missing `-o` flag
- **WHEN** the user runs `sound-cabinet render <score.sc>` without `-o`
- **THEN** the program exits with a non-zero status
- **AND** prints a usage message naming the `-o <output.wav>` requirement

#### Scenario: Missing score path
- **WHEN** the user runs `sound-cabinet render -o out.wav`
- **THEN** the program exits with a non-zero status
- **AND** prints a usage message naming the score path requirement

### Requirement: Render prints integrated loudness, true peak, and per-voice levels

After rendering, the command SHALL print to stderr the integrated loudness in LUFS (per ITU-R BS.1770), the true peak in dBFS, and a per-voice level table showing RMS dB, peak dB, and a `Status` column.

#### Scenario: LUFS and peak printed
- **WHEN** a render completes successfully
- **THEN** stderr contains a line of the form `Integrated loudness: <N> LUFS`
- **AND** stderr contains a line of the form `True peak: <N> dBFS`

#### Scenario: Per-voice level table printed
- **WHEN** a render completes successfully and the score contained at least one voiced event
- **THEN** stderr contains a table with columns `Voice`, `RMS`, `Peak`, `Status`
- **AND** rows are sorted by RMS descending (loudest voice first)
- **AND** each row's `Status` is `INAUDIBLE` (RMS < -60 dB), `Very quiet` (RMS < -40 dB), `Quiet` (RMS < -24 dB), `Dominant` (peak > -1 dB), or `OK`

### Requirement: `--lufs` flag normalizes integrated loudness to a target

The `--lufs <target>` flag SHALL cause the renderer to apply a single gain correction after rendering so that the output's integrated loudness equals the target value. If the resulting true peak would exceed -0.1 dBFS, the renderer SHALL emit a clipping-risk warning to stderr.

#### Scenario: Normalize to -14 LUFS (Spotify/YouTube)
- **WHEN** the user runs `render <score.sc> -o out.wav --lufs -14`
- **THEN** the output WAV's integrated loudness is -14 LUFS (±0.1 dB tolerance after a single-pass gain)
- **AND** the reported true peak reflects the post-normalization value

#### Scenario: Normalization would clip
- **WHEN** `--lufs <target>` is given AND the post-normalization true peak would exceed -0.1 dBFS
- **THEN** stderr contains a clipping-risk warning naming the resulting peak

#### Scenario: Invalid `--lufs` value
- **WHEN** the user supplies `--lufs <non-numeric>`
- **THEN** the program exits non-zero with an error naming `--lufs requires a number`

### Requirement: `--solo` flag mutes all but the named voices

The `--solo <voices>` flag SHALL accept a comma-separated list of voice names and SHALL cause the engine to render only events whose voice label matches one of the listed names. All other voices SHALL be muted. The flag SHALL emit `Solo: <voices>` to stderr at render start.

#### Scenario: Solo a single voice
- **WHEN** the user runs `render <score.sc> -o out.wav --solo bass`
- **THEN** the rendered WAV contains audio only from events labeled `bass`
- **AND** stderr contains the line `Solo: bass`

#### Scenario: Solo multiple voices
- **WHEN** the user runs `render <score.sc> -o out.wav --solo bass,melody`
- **THEN** only events labeled `bass` or `melody` are audible in the WAV
- **AND** stderr contains the line `Solo: bass, melody`

### Requirement: `--compress` and `--ceiling` override master bus settings from CLI

The `--compress` and `--ceiling` flags SHALL override any `master compress` and `master ceiling` directives in the score for this render only. `--compress` accepts either a single amount (e.g. `1.0`) or a comma-separated tuple of `threshold,ratio` or `threshold,ratio,attack,release`. `--ceiling` takes a single dBFS value.

#### Scenario: Override compression amount
- **WHEN** the user runs `render <score.sc> -o out.wav --compress 2.0`
- **THEN** the master compressor uses the amount 2.0 regardless of any `master compress` directive in the score

#### Scenario: Override compression with explicit parameters
- **WHEN** the user runs `--compress -18,2,0.05,0.2`
- **THEN** the master compressor uses threshold -18 dB, ratio 2:1, attack 50ms, release 200ms

#### Scenario: Override ceiling
- **WHEN** the user runs `--ceiling -1.0`
- **THEN** the master limiter ceiling is -1.0 dBFS regardless of any `master ceiling` directive in the score

#### Scenario: Bypass compression with `--compress 0`
- **WHEN** the user runs `--compress 0`
- **THEN** the master compressor is bypassed (amount = 0)

#### Scenario: Invalid `--compress` value count
- **WHEN** `--compress` is given a comma-separated list other than 1, 2, or 4 values
- **THEN** the program exits non-zero with `--compress: expected 1, 2, or 4 values`

### Requirement: Score's master bus chain is applied during render

The render SHALL apply the score's full master bus chain (HP 30 Hz → LP 18 kHz → user chain → brick-wall limiter) to the output before writing the WAV. CLI overrides (`--compress`, `--ceiling`, `--solo`) SHALL be applied before rendering begins.

#### Scenario: Master chain is applied
- **WHEN** the score includes `master chain compress(1.0) >> saturate(0.5)` and the user runs `render`
- **THEN** the WAV output reflects compression then saturation in the master path
- **AND** the always-present HP/LP bookends and limiter are also applied
