# Algorithmic Instrument Generation

Design document for a trait-driven instrument synthesis system that builds
playable instruments from high-level descriptive vocabulary rather than
explicit signal chains.

## Motivation

Building a convincing instrument in Sound Cabinet requires deep knowledge of
synthesis: choosing oscillators, tuning filter cutoffs, shaping envelopes,
layering harmonics, adding effects.  The piano took extended iteration ---
multiple oscillator layers, carefully tuned decay curves, per-register
filtering, detuning for warmth.  A nylon guitar, a marimba, a bowed cello
each require similar effort.

But instruments cluster into families with shared acoustic properties.  All
hammered strings share an impulse excitation, a resonant body, and a decay
envelope.  All bowed strings share sustained excitation with noise content
and a resonant body.  All plucked strings share a bright attack that mellows
as it decays.  The differences within a family are parameter variations, not
architectural changes.

This means we can encode instrument *archetypes* as parameterized templates,
then describe specific instruments as trait combinations that select and tune
those templates.

## Trait Vocabulary

An instrument is described by combining traits from several categories.
The generator resolves traits into a concrete signal chain.

### Excitation (how energy enters the system)

| Trait | Acoustic meaning | Synthesis mapping |
|-------|-----------------|-------------------|
| `hammered` | Impulse strike (piano, dulcimer, marimba) | Short noise burst or impulse into resonator |
| `plucked` | String pulled and released (guitar, harp, koto) | Karplus-Strong or bright impulse + fast LP sweep |
| `bowed` | Sustained friction (violin, cello, erhu) | Sawtooth-ish oscillator with noise component |
| `blown` | Air column (flute, clarinet, trumpet) | Filtered noise or pulse wave, breath envelope |
| `struck` | Percussive hit (drum, bell, gong) | Noise burst or impulse, inharmonic partials |
| `electronic` | No acoustic model (synth pad, lead) | Raw oscillator(s), no body simulation |

### Sustain behavior

| Trait | Meaning | Synthesis mapping |
|-------|---------|-------------------|
| `decaying` | Energy dissipates after excitation (piano, guitar) | Exponential decay envelope |
| `sustained` | Energy maintained while playing (organ, bowed, blown) | Flat sustain with attack/release |
| `swelling` | Gradual onset (pad, bowed crescendo) | Slow attack envelope |
| `percussive` | Very fast decay (click, woodblock, rimshot) | Ultra-short envelope, minimal sustain |

### Body / resonance character

| Trait | Meaning | Synthesis mapping |
|-------|---------|-------------------|
| `woody` | Warm, mid-heavy (acoustic guitar, marimba) | Bandpass around 200--800 Hz, gentle rolloff |
| `boxy` | Hollow resonance (old piano, toy piano) | Narrow bandpass, some resonance |
| `metallic` | Bright, inharmonic (bell, cymbal, steel drum) | Inharmonic partials, slow high-freq decay |
| `glassy` | Pure, few harmonics (vibraphone, celesta) | Near-sine with gentle harmonics |
| `airy` | Breathy, noise component (flute, shakuhachi) | Mix filtered noise with tone |
| `tight` | Dry, damped (muted guitar, clavinet) | Fast high-freq rolloff, short decay |
| `open` | Full resonance, long sustain (grand piano, gong) | Wide frequency range, slow decay |

### Register / brightness

| Trait | Meaning | Synthesis mapping |
|-------|---------|-------------------|
| `bright` | Emphasis on upper harmonics | Higher filter cutoff, less LP filtering |
| `dark` | Rolled-off highs (felt piano, nylon guitar) | Lower filter cutoff |
| `plinky` | Bright transient, dark sustain | Filter envelope: high cutoff -> low cutoff |
| `warm` | Even harmonics emphasized | Slight saturation / even-harmonic distortion |
| `thin` | Few partials, narrow spectrum | Sine-heavy oscillator mix |
| `rich` | Many partials, full spectrum | Saw or complex wave, minimal filtering |

### Texture modifiers

| Trait | Meaning | Synthesis mapping |
|-------|---------|-------------------|
| `detuned` | Slight pitch spread (chorus-like) | Multiple oscillators with cent offsets |
| `noisy` | Significant noise floor | Mix pink/white noise into signal |
| `clean` | Minimal harmonic distortion | Pure oscillators, no saturation |
| `gritty` | Distorted, rough | Soft-clip or wavefold after oscillator |
| `shimmery` | High-frequency movement | Modulated high-freq content, chorus |
| `muted` | Damped, as if physically stopped | Strong LP filter + fast decay |

## Resolution: Traits to Signal Chain

The generator maps trait combinations to a synthesis architecture using
rules, not neural networks.  The process:

1. **Select excitation model** from the excitation trait
2. **Shape envelope** from sustain behavior
3. **Build body resonance** from body traits (filter type, cutoff, Q)
4. **Set harmonic content** from register/brightness traits
5. **Apply texture modifiers** as post-processing

### Example: "plucked, decaying, woody, plinky"

```
Excitation: plucked -> bright impulse (noise burst, ~5ms)
Envelope:   decaying -> exponential decay, ~2s at middle register
Body:       woody -> bandpass emphasis 200-800 Hz, gentle Q
Brightness: plinky -> filter envelope from freq*8 down to freq*1.5

Result signal chain:
  noise(0.005) >> lowpass(freq * 8 -> freq * 1.5, 0.6) >> bandpass(500, 0.5) >> decay(8)
```

This is recognizably an acoustic guitar-like sound.  Not a *specific*
acoustic guitar, but something in the right family that a composer can
use immediately and refine if needed.

### Example: "hammered, decaying, boxy, bright, detuned"

```
Excitation: hammered -> impulse (shorter than plucked, sharper transient)
Envelope:   decaying -> exponential decay, register-dependent (~3s mid)
Body:       boxy -> narrow bandpass with moderate resonance
Brightness: bright -> higher initial filter cutoff
Texture:    detuned -> two oscillator layers, +/- 3 cents

Result signal chain:
  (saw(freq * 1.0017) + saw(freq * 0.9983)) * 0.5
    >> lowpass(freq * 6 -> freq * 2, 0.4)
    >> bandpass(800, 1.2)
    >> decay(10)
```

This produces something in the upright-piano / honky-tonk family.

### Example: "blown, sustained, airy, warm"

```
Excitation: blown -> filtered noise + sine fundamental
Envelope:   sustained -> ADSR with gentle attack (~50ms), full sustain
Body:       airy -> pink noise mixed at 15-20% level
Brightness: warm -> even harmonics via gentle saturation

Result signal chain:
  (sine(freq) + 0.15 * pink() >> bandpass(freq * 2, 0.8))
    >> distort(1.3)
    >> swell(0.05, 0.1)
```

A flute-ish or recorder-ish sound.

## Archetype Templates

Rather than resolving every trait combination from scratch, the generator
uses archetype templates --- pre-built signal chain skeletons for common
instrument families.  Traits then *tune* the template parameters.

```yaml
# archetypes/plucked-string.yaml
name: Plucked String
excitation: [plucked]
compatible_sustain: [decaying, percussive]
compatible_body: [woody, tight, metallic, open]

template: |
  {noise_mix} * noise({attack_ms})
    >> lowpass(freq * {cutoff_mult_start} -> freq * {cutoff_mult_end}, {filter_q})
    >> {body_filter}
    >> decay({decay_rate})

defaults:
  attack_ms: 0.003
  cutoff_mult_start: 8
  cutoff_mult_end: 1.5
  filter_q: 0.5
  decay_rate: 8

trait_overrides:
  woody:
    body_filter: "bandpass(500, 0.5)"
    cutoff_mult_end: 1.5
  metallic:
    body_filter: "highpass(2000, 0.3)"
    cutoff_mult_end: 3.0
    decay_rate: 12
  tight:
    body_filter: "lowpass(1200, 0.3)"
    decay_rate: 4
  bright:
    cutoff_mult_start: 12
  dark:
    cutoff_mult_start: 4
    cutoff_mult_end: 1.0
  detuned:
    noise_mix: "(saw(freq * 1.002) + saw(freq * 0.998)) * 0.5 +"
```

This lets us define maybe 6--8 archetypes (plucked string, hammered string,
bowed string, blown pipe, struck percussion, struck bell, electronic lead,
electronic pad) and get dozens of usable instruments through trait
combination.

## CLI Interface

```bash
# Describe what you want
sound-cabinet voice-gen \
  --traits "plucked, decaying, woody, plinky" \
  --name nylon_guitar \
  -o voices/nylon.sc

# Generate several variations
sound-cabinet voice-gen \
  --traits "hammered, decaying, boxy, bright" \
  --name piano \
  --variations 5 \
  -o voices/pianos.sc

# List available traits
sound-cabinet voice-gen --list-traits

# Show what archetype a trait set resolves to
sound-cabinet voice-gen --traits "bowed, sustained, warm" --explain
```

### Output

```sc
// Generated by: plucked + decaying + woody + plinky
instrument nylon_guitar = noise(0.003)
  >> lowpass(freq * 8 -> freq * 1.5, 0.5)
  >> bandpass(500, 0.5)
  >> decay(8)
```

The output is a standard `.sc` instrument definition.  The composer can use
it as-is, or open it up and tweak the parameters manually.  The generated
code is not opaque --- it's the same syntax a human would write.

## Relationship to Phrase Generation

Instrument generation and phrase generation are complementary:

- Phrase generation answers: "What notes should I play?"
- Instrument generation answers: "What should it sound like?"

Combined workflow:

1. `voice-gen --traits "plucked, woody, bright" -o voices/guitar.sc`
2. `generate --pattern walking-jazz --key D --voice guitar -o phrases.sc`
3. Listen, pick favorites, assemble

Or for exploration: generate 5 instrument variations and 10 phrase variations,
mix and match.  50 combinations from minimal input.

## Open Questions

- **Trait conflicts**: What happens with "metallic, woody"?  These are
  acoustically contradictory.  Options: error, pick the first one, blend
  (metallic body with woody warmth).  Probably blend with a warning.

- **Register-dependent behavior**: Real instruments sound different across
  their range (piano bass vs. treble).  Should traits accept register
  qualifiers?  `"bright(high), warm(low)"`.  Or should the archetype
  handle this automatically?

- **Layering**: Some instruments are fundamentally layered (piano = hammer
  noise + string resonance + body resonance).  A single signal chain may
  not capture this.  The template system could support parallel paths,
  pending the parallel signal routing feature in the roadmap.

- **Taste parameter**: A numeric "character" or "personality" dial (0.0 =
  textbook clean, 1.0 = exaggerated/characterful) that scales how
  aggressively traits are applied.  Low values for background instruments,
  high values for leads and solos.

- **Learning from refinement**: When a composer takes a generated instrument
  and tweaks it (changes the decay from 8 to 12, adjusts the filter Q),
  could those tweaks inform the archetype defaults over time?
