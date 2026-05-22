# audio-engine Specification

## Purpose
Define the synthesis primitives that compose a signal graph in Sound Cabinet expressions: oscillators, custom waveforms, filters, envelopes, effects, panning, chord generation, the arpeggiator, signal-graph operators, and parameter automation. The DSL surface that names and composes these primitives is defined in [[dsl-syntax]]; the master bus that all rendered audio passes through afterward is defined in [[master-bus]].

All synthesis SHALL run at 44.1 kHz sample rate. All output SHALL be stereo (2-channel). Voices without an explicit `pan()` SHALL play centered (equal energy to both channels).

## Requirements

### Requirement: Oscillator primitives

The expression language SHALL provide these oscillator function calls. Each accepts a frequency in Hz (or a note name resolved to Hz) and produces a signal in the range approximately [-1.0, +1.0]:

| Function | Waveform | Character |
|---|---|---|
| `sine(<freq>)` | Pure sine | Clean fundamental |
| `saw(<freq>)` | Sawtooth | Bright, buzzy, rich harmonics |
| `triangle(<freq>)` | Triangle | Softer than saw |
| `square(<freq>)` | Square (50% duty) | Hollow, woody |
| `pulse(<freq>, <width>)` | Pulse with variable width | `width` 0.0–1.0; 0.5 = square, < 0.5 = thinner/nasal, > 0.5 = brighter |
| `noise()` / `white()` | White noise | Flat spectrum |
| `pink()` | Pink noise | ~3 dB/octave HF rolloff |
| `brown()` | Brown noise | ~6 dB/octave HF rolloff (deep rumble) |

`noise()` and `white()` SHALL be aliases producing identical output. Pulse width SHALL be a number in [0.0, 1.0]; values outside this range SHALL be clamped.

#### Scenario: Sine oscillator
- **WHEN** the expression `sine(A4)` is rendered
- **THEN** the output is a sine wave at 440 Hz

#### Scenario: Square via pulse
- **WHEN** the expression `pulse(C3, 0.5)` is rendered
- **THEN** the output is functionally equivalent to `square(C3)` (50% duty cycle)

#### Scenario: noise() and white() are aliases
- **WHEN** the user writes `noise()` or `white()`
- **THEN** both produce statistically identical white noise

### Requirement: Custom wavetable oscillators

A `wave <name> = [<n1>, <n2>, ...]` declaration (defined in [[dsl-syntax]]) SHALL produce a callable oscillator `<name>(<freq>)` that reads through the sample array as one cycle at the frequency specified. Linear interpolation SHALL be used between sample points. The array length SHALL determine bandlimit and character: shorter arrays produce more aliasing (8-bit feel); longer arrays produce smoother output. Asymmetric arrays SHALL produce even harmonics.

#### Scenario: Custom wave at a frequency
- **WHEN** the score contains `wave plateau = [0.0, 0.4, 0.8, 1.0, 1.0, 1.0, 0.8, 0.4, 0.0, -0.4, -0.8, -1.0, -1.0, -1.0, -0.8, -0.4]`
- **AND** the expression `plateau(C3)` is rendered
- **THEN** the 16-sample array is treated as one cycle at C3's frequency (~130.81 Hz)
- **AND** sample interpolation is linear between points

#### Scenario: Custom waves compose with effects, arp, and instruments
- **WHEN** a custom wave is used as an oscillator in an `instrument` definition, in an `arp(...)` call, or as the source of any effect chain
- **THEN** the wave is treated identically to a built-in oscillator (e.g. `saw`)

### Requirement: Filter primitives

The expression language SHALL provide these filter function calls. Filters process the incoming signal and SHALL be chained after a source with `>>`:

- `lowpass(<freq>, <q>)` — Cuts frequencies above `<freq>` Hz. `<q>` controls resonance (≈ 0.5 = gentle, ≈ 2.0 = sharp peak).
- `lowpass(<freq>, <q>, <mix>)` — Same with dry/wet blend; `<mix>` 0.0 = all dry, 1.0 = fully filtered.
- `highpass(<freq>, <q>)` — Cuts frequencies below `<freq>` Hz with the same `<q>` behavior.
- `highpass(<freq>, <q>, <mix>)` — Same with dry/wet blend.

Filter frequencies (and pulse width) SHALL accept the parameter-sweep form `<start> -> <end>` to linearly interpolate over the duration of the enclosing event (see [parameter automation requirement](#requirement-parameter-automation-with-the---sweep-operator)).

#### Scenario: Lowpass filter
- **WHEN** the expression `saw(C3) >> lowpass(800, 0.7)` is rendered
- **THEN** content above ~800 Hz is attenuated with gentle resonance at the cutoff

#### Scenario: Filter dry/wet mix
- **WHEN** the expression `saw(C3) >> lowpass(800, 0.7, 0.5)` is rendered
- **THEN** the output is a 50/50 mix of the dry saw and the fully-filtered saw

### Requirement: Envelope primitives

The expression language SHALL provide these envelope function calls:

- `decay(<rate>)` — Exponential amplitude decay: amplitude drops as `e^(-rate * t)` over event duration. Higher `<rate>` = faster decay.
- `swell(<attack>, <release>)` — Fade in linearly over `<attack>` seconds at the start of the event, then fade out linearly over `<release>` seconds at the end of the event.

#### Scenario: Decay envelope
- **WHEN** the expression `sine(A4) >> decay(15)` is rendered for 1 beat
- **THEN** the amplitude starts at full and exponentially decays with rate 15

#### Scenario: Swell envelope
- **WHEN** the expression `pad >> swell(0.5, 2.0)` is rendered for 8 beats
- **THEN** the pad fades in over the first 0.5 s and fades out over the last 2.0 s

### Requirement: Effect primitives

The expression language SHALL provide these per-event effects. Each is chained after a source with `>>`. Parameter ranges and semantics:

| Function | Behavior |
|---|---|
| `lfo(<rate>, <depth>)` | Tremolo (amplitude modulation). `<rate>` in Hz; `<depth>` 0.0–1.0 |
| `distort(<amount>)` | Soft clipping (tanh saturation). ~1.0 = subtle warmth, ~4.0+ = heavy drive |
| `vibrato(<rate>, <depth>)` | Pitch wobble via modulated delay. `<rate>` Hz; `<depth>` samples |
| `chorus(<sep>, <var>, <freq>)` | Detuned copies for width. `<sep>` and `<var>` in seconds; `<freq>` Hz |
| `delay(<time>, <fb>, <mix>)` | Feedback delay with HF-damped feedback path. `<time>` s; `<fb>` 0.0–1.0; `<mix>` 0.0–1.0 |
| `reverb(<size>, <damp>, <mix>)` | Freeverb algorithmic reverb. `<size>` 0.0–1.0 room size; `<damp>` 0.0–1.0 HF absorption; `<mix>` 0.0–1.0 dry/wet |
| `compress(<thresh>, <ratio>, <atk>, <rel>)` | Dynamic-range compression. `<thresh>` dB; `<ratio>` e.g. 4 = 4:1; `<atk>`/`<rel>` seconds. 6 dB soft knee |
| `compress(<thresh>, <ratio>, <atk>, <rel>, up)` | Upward compression: raise content below threshold instead of attenuating content above |
| `expand(<thresh>, <ratio>, [<atk>, <rel>])` | Downward expansion below threshold. Defaults: `<atk>` 0.01 s, `<rel>` 0.1 s. 6 dB soft knee |
| `crush(<bits>)` | Bit-depth reduction. 8 = retro, 10 = subtle, 4 = destroyed |
| `decimate(<factor>)` | Sample-rate reduction. 2 = half rate, 8 = heavy digital |
| `degrade(<amount>)` | Combined lowpass + decimate + crush + noise (tape character). 0.3 warm, 0.6 worn, 1.0 destroyed |
| `loudness(<freq>)` | ISO 226 equal-loudness compensation. Frequency-dependent gain (1 kHz = 0 dB). Useful inside instruments where `<freq>` is `freq` |
| `eq(<freq>, <gain_db>, <q>)` | Parametric peak (bell) EQ. `<q>` is bandwidth; 0.5 = wide, 3.0 = narrow |
| `eq(<freq>, <gain_db>, low)` | Low shelf — boost/cut everything below `<freq>` (Q = 0.707) |
| `eq(<freq>, <gain_db>, high)` | High shelf — boost/cut everything above `<freq>` (Q = 0.707) |
| `pan(<pos>)` | Equal-power stereo pan. `<pos>` -1.0 = full left, 0.0 = center, 1.0 = full right |
| `bus(<name>)` | Tag this event's output to a named sidechain bus (no audible effect on its own) |
| `excite(<freq>, <amount>)` | Harmonic exciter. Saturates content above `<freq>` and blends back at `<amount>` 0.0–1.0 |
| `sidechain(<bus>[, <thresh>, <ratio>, <atk>, <rel>])` | Duck this signal based on the named bus's level. Defaults: -20 dB threshold, 4:1 ratio, 0.01 s attack, 0.1 s release |

#### Scenario: Reverb effect
- **WHEN** the expression `saw(C4) >> reverb(0.8, 0.4, 0.3)` is rendered
- **THEN** Freeverb is applied with room size 0.8, HF damping 0.4, 30% wet mix

#### Scenario: Per-event compressor with attack/release
- **WHEN** the expression `saw(C2) >> compress(-15, 4, 0.01, 0.1)` is rendered
- **THEN** signal above -15 dB is compressed at 4:1 with 10 ms attack and 100 ms release, using a 6 dB soft knee

#### Scenario: Per-event upward compression
- **WHEN** the expression `saw(C3) >> compress(-30, 2, 0.01, 0.1, up)` is rendered
- **THEN** the compressor operates in upward mode (raises content below -30 dB)

#### Scenario: Per-event expander with defaults
- **WHEN** the expression `pad >> expand(-40, 1.5)` is rendered
- **THEN** content below -40 dB is expanded at 1.5:1 with default attack 10 ms and release 100 ms

#### Scenario: Parametric peak EQ
- **WHEN** the expression `saw(C3) >> eq(400, -3, 1.5)` is rendered
- **THEN** a -3 dB peak cut at 400 Hz with Q=1.5 is applied

#### Scenario: Low/high shelf EQ
- **WHEN** the expression `saw(C2) >> eq(80, 4, low)` is rendered
- **THEN** a +4 dB low shelf below 80 Hz is applied with Q = 0.707 (Butterworth)
- **AND** `eq(10000, 2, high)` applies a +2 dB shelf above 10 kHz

#### Scenario: Bus and sidechain
- **WHEN** event A is `kick >> bus(drums) for 0.5 beats`
- **AND** event B is `pad >> sidechain(drums) for 8 beats`
- **THEN** event A's output is tagged to the `drums` bus
- **AND** event B's level ducks whenever the `drums` bus exceeds -20 dB, with default 4:1 ratio, 10 ms attack, 100 ms release

#### Scenario: Sidechain with explicit parameters
- **WHEN** event B is `pad >> sidechain(drums, -20, 4, 0.01, 0.1)`
- **THEN** the same defaults are made explicit (functionally identical to `sidechain(drums)`)

#### Scenario: Harmonic exciter
- **WHEN** the expression `saw(C3) >> excite(3000, 0.5)` is rendered
- **THEN** content above 3 kHz is isolated, saturated to generate new harmonics, and blended back at 50% level

#### Scenario: Equal-loudness compensation inside an instrument
- **WHEN** an instrument is defined as `instrument lead = saw(freq) >> decay(6) >> loudness(freq)`
- **AND** invoked as `lead(C2)` (~65 Hz)
- **THEN** the loudness stage applies ~+8 dB compensation
- **AND** for `lead(A4)` (440 Hz) the compensation is ~+0.3 dB

#### Scenario: Bit crush
- **WHEN** the expression `saw(C3) >> crush(8)` is rendered
- **THEN** the signal is quantized to ~8-bit resolution

#### Scenario: Decimate (sample-rate reduction)
- **WHEN** the expression `sine(A4) >> decimate(4)` is rendered
- **THEN** the effective sample rate is reduced by a factor of 4

#### Scenario: Combined degrade
- **WHEN** the expression `triangle(C4) >> degrade(0.5)` is rendered
- **THEN** a coordinated combination of lowpass, decimate, crush, and noise is applied at 50% strength

### Requirement: Stereo panning is the final pre-bus stage

`pan(<pos>)` SHALL be the last per-event stage before the master bus (or just before `bus()`/`sidechain()`). Stages before `pan()` process in mono; `pan()` converts the mono signal to stereo using equal-power panning so perceived loudness is constant across the field.

A voice without an explicit `pan()` SHALL play centered (equal in both channels).

#### Scenario: Static pan
- **WHEN** the expression `noise() >> highpass(8000) >> decay(25) >> pan(0.7)` is rendered
- **THEN** the signal is processed in mono through the chain
- **AND** the final stage places it at position +0.7 (right of center) using equal-power panning

#### Scenario: Pan sweep
- **WHEN** the expression `saw(440) >> pan(-1.0 -> 1.0)` is rendered for 8 beats
- **THEN** the pan position sweeps linearly from full left at beat 0 to full right at beat 8

#### Scenario: No explicit pan
- **WHEN** an expression has no `pan(...)` stage
- **THEN** the output is centered (equal energy in both channels)

### Requirement: `chord(<name>)` generates a summed-saw chord

`chord(<name>)` SHALL produce a summed set of saw oscillators corresponding to the named chord. The chord name SHALL follow the format defined in [[dsl-syntax]] (`Root[Accidental][Octave]:Quality`). The result SHALL be usable anywhere an oscillator is valid.

#### Scenario: Minor 7th chord
- **WHEN** the expression `chord(C:m7)` is used as an oscillator source
- **THEN** four saw oscillators (root, b3, 5, b7 of C minor) are summed at octave 4

#### Scenario: Chord with explicit octave
- **WHEN** the expression `chord(Ab3:maj7)` is used
- **THEN** four saw oscillators at A-flat major 7th at octave 3 are summed

### Requirement: Arpeggiator splits a voice into a note sequence

`arp(<notes...>, <rate>[, <options...>])` SHALL split the source signal into a sequence of notes over the duration of the enclosing event. Notes MAY be specified as individual note-name frequencies or as a chord name (which is expanded into its constituent notes). `<rate>` SHALL be notes per beat.

Options (any order after the rate):

- Direction: `up` (default), `down`, `updown`, `random`
- Octave spanning: `up<N>`, `down<N>`, `updown<N>` (where `<N>` is the number of octaves)
- `gate, <value>` — note length as a fraction of step length (default 1.0; < 1.0 staccato; > 1.0 legato overlap)
- `accent, <N>` — boost every Nth note (1.5× gain on accented, 0.7× on unaccented)
- `steps, <pattern>` — rhythmic gating where `x` plays and `_` rests; pattern cycles
- Speed ramp via `<rate1> -> <rate2>` for the rate argument

The arp SHALL work with voices, instruments, and wavetables. Filter cutoffs and other `freq`-relative parameters in the source instrument SHALL track each arp note (the arp substitutes `freq` per note).

#### Scenario: Default ascending arp from a chord
- **WHEN** the expression `pluck >> arp(C:m7, 4)` is rendered for 4 beats
- **THEN** four notes per beat play in ascending order through C minor 7 (C, Eb, G, Bb), looping

#### Scenario: Descending across two octaves
- **WHEN** the expression `pluck >> arp(C:m7, 4, down2)` is used
- **THEN** notes play descending across two octaves before repeating

#### Scenario: Ping-pong (updown)
- **WHEN** the expression uses `updown`
- **THEN** notes ascend then descend, looping the up-down cycle

#### Scenario: Gate and accent combined
- **WHEN** the expression `pluck >> arp(C:m7, 8, accent, 4, gate, 0.5)` is used
- **THEN** notes play at 8 per beat at 50% step length (staccato)
- **AND** every 4th note has 1.5× gain; other notes have 0.7× gain

#### Scenario: Step pattern (rhythmic gating)
- **WHEN** the expression `pluck >> arp(C:m7, 8, steps, x_x_xx_x)` is used
- **THEN** the pattern `x_x_xx_x` cycles; `x` plays a note, `_` rests

#### Scenario: Speed ramp
- **WHEN** the expression `pluck >> arp(C:m7, 2 -> 8)` is rendered for 8 beats
- **THEN** the rate sweeps linearly from 2 notes/beat to 8 notes/beat

### Requirement: Signal-graph operators

The expression language SHALL support these operators with the precedence defined in [[dsl-syntax]]:

| Operator | Meaning |
|---|---|
| `>>` | Pipe — output of left feeds into right |
| `+` | Mix — add signals sample-wise |
| `-` | Subtract signals sample-wise |
| `*` | Multiply — scale by a constant or another signal |
| `/` | Divide — primarily useful in `instrument` definitions for inverse-frequency expressions |

#### Scenario: Pipe chain
- **WHEN** the expression `saw(C3) >> lowpass(800, 0.7) >> reverb(0.6, 0.4, 0.3)` is rendered
- **THEN** the saw passes through lowpass then reverb in order

#### Scenario: Mix two oscillators
- **WHEN** the expression `(saw(C3) + 0.5 * sine(C4)) >> lowpass(2000, 0.7)` is rendered
- **THEN** a full saw and a half-amplitude sine are summed before entering the lowpass

#### Scenario: Constant scaling
- **WHEN** the expression `0.3 * plateau(C3) >> lowpass(2000, 0.7)` is rendered
- **THEN** the plateau output is scaled to 30% before the lowpass

### Requirement: Parameter automation with the `->` sweep operator

The `<a> -> <b>` operator SHALL be valid in any numeric argument position of an oscillator, filter, or effect call and SHALL produce a linear interpolation from `<a>` at the event's start to `<b>` at the event's end. The sweep duration SHALL equal the enclosing `for N beats` event's duration.

#### Scenario: Filter sweep
- **WHEN** the expression `saw(C3) >> lowpass(800 -> 4000, 0.7)` is rendered for 4 beats
- **THEN** the lowpass cutoff sweeps linearly from 800 Hz at beat 0 to 4000 Hz at beat 4

#### Scenario: Pulse-width modulation sweep
- **WHEN** the expression `pulse(C3, 0.1 -> 0.9)` is rendered for 4 beats
- **THEN** the pulse width sweeps linearly from 0.1 to 0.9 over the event

#### Scenario: Pan sweep
- **WHEN** the expression `saw(440) >> pan(-1.0 -> 1.0)` is rendered for 8 beats
- **THEN** the pan position sweeps from full left at beat 0 to full right at beat 8

#### Scenario: Speed ramp inside arp
- **WHEN** an arp uses `<rate1> -> <rate2>` (e.g. `arp(C:m7, 2 -> 8)`)
- **THEN** the arp rate sweeps linearly across the enclosing event's duration

### Requirement: `freq` is a reserved variable inside `instrument` definitions

Inside an `instrument` definition body, the identifier `freq` SHALL be a placeholder for the actual frequency in Hz passed at invocation time. `freq` MAY appear anywhere a number is valid (oscillator arguments, filter cutoffs, arithmetic). Constant expressions referencing `freq` (e.g. `freq * 4`) SHALL be evaluated at substitution time when the instrument is invoked. Outside of `instrument` bodies, `freq` is not bound.

#### Scenario: freq substitution in an instrument
- **WHEN** the instrument is `instrument piano = saw(freq) >> lowpass(freq * 4, 0.7) >> decay(8)`
- **AND** the call is `piano(C4)` (~261.63 Hz)
- **THEN** the oscillator runs at 261.63 Hz
- **AND** the lowpass cutoff is 261.63 × 4 ≈ 1046 Hz

#### Scenario: freq in `arp` preserves filter tracking
- **WHEN** the instrument uses `freq`-relative filter cutoffs AND is wrapped in `>> arp(...)`
- **THEN** the filter cutoffs track each arp note's frequency

### Requirement: `normalize <name> <target>` adjusts the average output level of an instrument or voice

When a `normalize <name> <target>` directive is present (see [[dsl-syntax]]), the engine SHALL render short test tones at multiple frequencies (covering at least C2 through C6) through the named source, measure the average RMS, and apply a gain correction so that the source's output averages the specified target (0.0–1.0 linear scale, where 1.0 = full scale and 0.5 = -6 dB).

#### Scenario: Normalize an instrument
- **WHEN** the score contains `normalize bass 0.5`
- **THEN** the engine renders test tones across C2–C6 through `bass`
- **AND** computes a single gain correction
- **AND** all subsequent uses of `bass` are scaled by that gain so the average RMS is approximately -6 dB
