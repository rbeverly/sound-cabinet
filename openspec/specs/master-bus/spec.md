# master-bus Specification

## Purpose
TBD - created by archiving change initial-spec-baseline. Update Purpose after archive.
## Requirements
### Requirement: Always-present master bus bookends

Every audio output path SHALL apply, in order:

1. A 2nd-order Butterworth highpass at 30 Hz
2. A 2nd-order Butterworth lowpass at 18 kHz
3. The user-definable processing chain (default: `compress(1.0)`)
4. A brick-wall limiter at -0.3 dBFS with 5 ms lookahead

The HP and LP filters and the brick-wall limiter SHALL be always present and SHALL NOT be removable or reorderable by user code or CLI flags.

#### Scenario: Bookends apply to every output mode
- **WHEN** any of `render`, `play`, `watch`, `piano`, or `stream` is invoked
- **THEN** the rendered/played audio passes through HP 30 Hz, LP 18 kHz, the user chain, and the limiter in that order

#### Scenario: Default user chain is `compress(1.0)`
- **WHEN** a score specifies no `master chain` and no individual `master ...` directives
- **THEN** the user-definable chain consists of a single `compress` stage at amount 1.0

### Requirement: `master chain` directive defines arbitrary effect ordering

The `master chain <stage1> >> <stage2> >> ...` directive SHALL fully replace the user-definable portion of the master chain with an ordered, possibly repeated sequence of stages. Each stage SHALL be one of: `compress(...)`, `saturate(...)`, `eq(...)`, `excite(...)`, `expand(...)`, or `multiband(...)`. Stages run left to right, connected by `>>`. Duplicates SHALL be permitted.

#### Scenario: Custom chain replaces default
- **WHEN** the score contains `master chain eq(80, -3, low) >> compress(1.0) >> saturate(0.3)`
- **THEN** the user chain consists of those three stages in that order
- **AND** the default `compress(1.0)` is NOT separately applied

#### Scenario: Serial compression (duplicates allowed)
- **WHEN** the score contains `master chain compress(0.5) >> compress(0.5)`
- **THEN** two distinct compressor stages run in sequence

### Requirement: Individual `master <effect>` directives build the chain in source order

If a score does not declare `master chain`, individual `master <effect>` directives (`master compress`, `master saturate`, `master excite`, `master curve`, `master multiband`, `master expand`) SHALL be inserted into the user chain in the order they appear in the score. The order in source SHALL be the order in the chain.

#### Scenario: Individual directives compose in source order
- **WHEN** the score contains, in order, `master compress 1.0`, `master saturate 0.5`, `master excite 4000 0.3`
- **THEN** the user chain is `compress(1.0) → saturate(0.5) → excite(4000, 0.3)`

### Requirement: `master compress` configures the master compressor

`master compress <amount>` SHALL set the master compressor with an amount value where the amount maps to a threshold/ratio pair:

| Amount | Threshold | Ratio |
|---|---|---|
| 0.5 | -36 dB | 1.5:1 |
| 1.0 | -18 dB | 2:1 |
| 2.0 | -9 dB | 3:1 |
| 0 | bypass | bypass |

`master compress <threshold> <ratio>` SHALL set explicit dB threshold and ratio. `master compress <threshold> <ratio> <attack> <release>` SHALL additionally set explicit attack/release in seconds. A trailing `up` keyword SHALL select upward compression mode (raise quiet content) instead of the default downward mode.

The compressor SHALL use a 6 dB soft knee (Giannoulis/Massberg/Reiss, JAES 2012).

#### Scenario: Compress by amount
- **WHEN** the score contains `master compress 2.0`
- **THEN** the master compressor uses threshold -9 dB, ratio 3:1, default attack/release

#### Scenario: Explicit compressor parameters
- **WHEN** the score contains `master compress -18 2 0.05 0.2`
- **THEN** the master compressor uses threshold -18 dB, ratio 2:1, attack 50ms, release 200ms

#### Scenario: Bypass with amount 0
- **WHEN** the score contains `master compress 0`
- **THEN** the master compressor is bypassed entirely

#### Scenario: Upward compression mode
- **WHEN** the score contains `master compress -30 2 0.01 0.1 up`
- **THEN** the master compressor operates in upward mode (raising signals below -30 dB toward unity instead of attenuating signals above the threshold)

### Requirement: `master expand` configures the master downward expander

`master expand <threshold> <ratio> [<attack> <release>]` SHALL set the master downward expander. Default attack is 0.01 s; default release is 0.1 s. The expander SHALL use a 6 dB soft knee. Signals below the threshold SHALL be attenuated by the given ratio.

#### Scenario: Default attack/release
- **WHEN** the score contains `master expand -30 2`
- **THEN** the master expander uses threshold -30 dB, ratio 2:1, attack 10 ms, release 100 ms

#### Scenario: Explicit attack/release
- **WHEN** the score contains `master expand -35 3 0.01 0.2`
- **THEN** the master expander uses threshold -35 dB, ratio 3:1, attack 10 ms, release 200 ms

### Requirement: `master saturate` applies a tanh soft clipper

`master saturate <amount>` SHALL insert a `tanh`-based waveshaper between the compressor and the limiter (or in the position dictated by `master chain`). The amount SHALL control drive level into the waveshaper, on a 0.0–1.0 scale. `master saturate 0` and `master saturate off` SHALL bypass it.

#### Scenario: Saturation drives the waveshaper
- **WHEN** the score contains `master saturate 0.5`
- **THEN** the master output passes through a tanh waveshaper at gentle saturation

#### Scenario: Bypass with off
- **WHEN** the score contains `master saturate off`
- **THEN** no saturation is applied

### Requirement: `master curve` shapes 3-band tonal balance

`master curve` SHALL apply a static 3-band EQ on the master:

- Low band: shelf at 120 Hz
- Mid band: peak (bell) at 1 kHz
- High band: shelf at 6 kHz

The directive SHALL accept either a preset name (`car`, `broadcast`, `bright`, `warm`, `flat`) or explicit per-band gains in dB.

Preset values:

| Preset | Low (120 Hz) | Mid (1 kHz) | High (6 kHz) |
|---|---|---|---|
| `car` | -4 dB | 0 dB | +3 dB |
| `broadcast` | -2 dB | 0 dB | -1 dB |
| `bright` | 0 dB | 0 dB | +3 dB |
| `warm` | +2 dB | 0 dB | -2 dB |
| `flat` | 0 dB | 0 dB | 0 dB |

Manual form: `master curve low <N>, mid <N>, high <N>` (values in dB).

#### Scenario: Preset curve
- **WHEN** the score contains `master curve car`
- **THEN** the master EQ applies -4 dB low shelf, 0 dB mid peak, +3 dB high shelf

#### Scenario: Manual per-band
- **WHEN** the score contains `master curve low -4, mid 0, high 3`
- **THEN** the master EQ applies -4 dB at 120 Hz (low shelf), 0 dB at 1 kHz (peak), +3 dB at 6 kHz (high shelf)

### Requirement: `master multiband` applies 3-band compression with LR4 crossovers

`master multiband <amount>` SHALL apply a 3-band compressor with 4th-order Linkwitz-Riley (LR4) crossovers and allpass delay compensation for phase coherence at the crossovers. Bands and per-band attack/release:

| Band | Frequency range | Attack | Release |
|---|---|---|---|
| Low | below 200 Hz | 15 ms | scaled |
| Mid | 200 Hz – 3 kHz | 5 ms | scaled |
| High | above 3 kHz | 1 ms | scaled |

`master multiband 0` and `master multiband off` SHALL bypass the multiband. `master multiband low <N>, mid <N>, high <N>` SHALL set per-band amounts independently.

#### Scenario: Single-amount multiband
- **WHEN** the score contains `master multiband 0.6`
- **THEN** all three bands compress at amount 0.6 with the band-specific attack/release values

#### Scenario: Per-band amounts
- **WHEN** the score contains `master multiband low 0.5, mid 0.3, high 0.2`
- **THEN** the low band compresses at 0.5, mid at 0.3, high at 0.2

#### Scenario: Bypass with off
- **WHEN** the score contains `master multiband off` or `master multiband 0`
- **THEN** all three bands are bypassed

### Requirement: `master excite` adds high-frequency harmonic content

`master excite <cutoff> <amount>` SHALL apply a harmonic exciter on the master: content above `<cutoff>` Hz is isolated, saturated to generate harmonics, and blended back into the output at level `<amount>` (0.0–1.0).

#### Scenario: Master exciter
- **WHEN** the score contains `master excite 4000 0.3`
- **THEN** content above 4 kHz is saturated and blended back at 30% level

### Requirement: `master gain` and `master ceiling` control overall level

`master gain <dB>` SHALL apply a gain (positive or negative dB) to the master signal before the limiter. `master ceiling <dBFS>` SHALL set the brick-wall limiter's ceiling. The default ceiling is -0.3 dBFS.

#### Scenario: Reduce overall level
- **WHEN** the score contains `master gain -6`
- **THEN** the master signal is attenuated by 6 dB before the limiter

#### Scenario: Custom limiter ceiling
- **WHEN** the score contains `master ceiling -1.0`
- **THEN** the brick-wall limiter caps peaks at -1.0 dBFS

### Requirement: Every `render` reports integrated loudness and true peak

After every render, the engine SHALL measure and print:
- Integrated loudness in LUFS per ITU-R BS.1770
- True peak in dBFS

#### Scenario: Render reports loudness metrics
- **WHEN** `render` completes
- **THEN** stderr contains `Integrated loudness: <N> LUFS`
- **AND** stderr contains `True peak: <N> dBFS`

### Requirement: `--lufs <target>` normalizes integrated loudness post-render

The `--lufs <target>` CLI flag (on `render`) SHALL apply a single gain correction after the full master chain so the output's integrated loudness equals the target value. If the resulting true peak would exceed -0.1 dBFS, a clipping warning SHALL be printed.

#### Scenario: Normalize to streaming target
- **WHEN** the user runs `render <score.sc> -o out.wav --lufs -14`
- **THEN** the output's integrated loudness is -14 LUFS (within rounding tolerance of a single gain pass)

#### Scenario: Normalization-induced clipping warning
- **WHEN** the post-normalization peak would exceed -0.1 dBFS
- **THEN** stderr contains a clipping warning naming the resulting peak

### Requirement: CLI overrides take precedence over score directives

When `render` is invoked with `--compress` or `--ceiling`, those CLI values SHALL override any matching `master compress` / `master ceiling` directives in the score, applying to the render only (not modifying the score on disk).

#### Scenario: CLI compress overrides score
- **WHEN** the score contains `master compress 1.0` AND the user runs `--compress 2.0`
- **THEN** the master compressor uses amount 2.0 for this render

#### Scenario: CLI ceiling overrides score
- **WHEN** the score contains `master ceiling -0.3` AND the user runs `--ceiling -1.0`
- **THEN** the limiter ceiling is -1.0 dBFS for this render

### Requirement: Realtime master bypass with auto volume matching

During `play` (and `piano`), the keys `m` and `\` SHALL toggle the user-definable portion of the master chain on/off. When bypassed, the RMS of the dry signal SHALL be auto-matched to the wet signal's RMS in real time so the comparison is not biased by loudness. The terminal SHALL display a live `[ GR -<N> dB ]` gain-reduction meter representing the combined instantaneous reduction from the master compressor, multiband, and limiter.

#### Scenario: Bypass toggle and volume match
- **GIVEN** playback is active with a non-empty user chain
- **WHEN** the user presses `m` or `\`
- **THEN** the user-definable portion of the chain is bypassed
- **AND** the dry RMS is matched to the wet RMS in real time

#### Scenario: Always-present bookends still apply when bypassed
- **WHEN** the user chain is bypassed via `m`/`\` (or via `test-master`)
- **THEN** the HP 30 Hz, LP 18 kHz, and brick-wall limiter still apply

