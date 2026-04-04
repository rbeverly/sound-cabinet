[← Back to README](../README.md)

# Master Bus & Loudness

Every render in Sound Cabinet passes through an automatic master bus chain before output. This applies to all modes: `render`, `play`, `watch`, `piano`, and `stream`.

## Master Bus Chain

The chain has always-present bookends (HP/LP filters and limiter) with a user-definable processing chain in between:

1. **Highpass at 30 Hz** -- removes inaudible sub-bass that eats headroom (Butterworth 2nd-order) *(always present)*
2. **Lowpass at 18 kHz** -- removes ultrasonic content from aliasing and filter resonance (Butterworth 2nd-order) *(always present)*
3. **User-definable chain** -- any combination and ordering of effects (default: `compress(1.0)`)
4. **Brick-wall limiter at -0.3 dBFS** -- prevents peaks from hitting 0 dBFS, with 5ms lookahead for clean transient handling *(always present)*

```
HP 30Hz → LP 18kHz → [ user-definable chain ] → Limiter
```

The HP/LP filters and limiter are always-present bookends that cannot be removed or reordered. Everything in between is fully configurable via the `master chain` directive or individual `master` commands.

## User-Definable Chain

The `master chain` directive lets you specify the exact processing order. Effects execute left to right, connected by `>>`. You can use any combination and ordering, including duplicates:

```sc
// User-definable master chain — effects execute left to right
master chain eq(80, -3, low) >> compress(1.0) >> eq(3000, 2, high) >> compress(0.5) >> saturate(0.3) >> excite(4000, 0.3)

// Serial compression (two gentle passes = more transparent than one heavy pass)
master chain compress(0.5) >> compress(0.5)

// Pre/post EQ with compression
master chain eq(80, -3, low) >> compress(1.0) >> eq(3000, 2, high)

// Clean then tighten (expand noise floor, then compress)
master chain expand(-35, 2) >> compress(1.0) >> saturate(0.3)
```

Individual `master` commands still work and build the chain in order of appearance:

```sc
master compress 1.0
master saturate 0.5
master excite 4000 0.3
```

If no `master chain` or individual `master` commands are specified, the default chain is `compress(1.0)`.

## Configuring Compression

The compressor uses a 6 dB soft knee (Giannoulis/Massberg/Reiss, JAES 2012) for smoother, more transparent dynamics control. The soft knee gradually transitions into compression around the threshold rather than applying an abrupt ratio change.

Control the master compressor from within a score using the `master compress` directive:

```sc
master compress 0.5                // gentle -- subtle dynamic tightening
master compress 1.0                // default -- standard mastering compression
master compress 2.0                // heavy -- loud, punchy, reduced dynamic range
master compress 0                  // off -- bypass compressor entirely
```

The `amount` value maps to threshold/ratio internally:

| Amount | Threshold | Ratio | Character |
|--------|-----------|-------|-----------|
| 0.5 | -36 dB | 1.5:1 | Gentle |
| 1.0 | -18 dB | 2:1 | Standard |
| 2.0 | -9 dB | 3:1 | Heavy |

Higher values produce louder, more compressed output at the cost of dynamic range.

For full control, specify threshold (dB), ratio, and optionally attack/release (seconds):

```sc
master compress -18 2              // explicit threshold (dB) and ratio
master compress -18 2 0.05 0.2     // threshold, ratio, attack (s), release (s)
```

A slow attack (50-100ms) lets transients punch through before compression engages.

## Master Expander

The expander reduces the level of signals below a threshold -- the opposite of a compressor. Use it to clean up the noise floor before compression, or to add definition to quiet passages by pushing them further down.

Like the compressor, the expander uses a 6 dB soft knee (Giannoulis/Massberg/Reiss, JAES 2012) for a smooth transition around the threshold.

```sc
master expand -30 2                       // threshold (dB), ratio
master expand -35 3 0.01 0.2             // threshold, ratio, attack (s), release (s)
```

Parameters:

- **threshold** -- dB level below which expansion kicks in (e.g. -30)
- **ratio** -- expansion ratio (e.g. 2 = 2:1 reduction below threshold)
- **attack** -- how fast the expander responds (seconds, default: 0.01)
- **release** -- how fast it recovers (seconds, default: 0.1)

| Threshold | Ratio | Character |
|-----------|-------|-----------|
| -40 dB | 1.5:1 | Gentle noise floor cleanup |
| -30 dB | 2:1 | Standard gating -- pushes quiet content down |
| -20 dB | 3:1 | Aggressive -- strong separation between signal and noise |

The expander is most effective when placed before the compressor in the chain. Compression raises the noise floor; expanding first cleans it up so the compressor has less noise to amplify:

```sc
// Clean then tighten
master chain expand(-35, 2) >> compress(1.0) >> saturate(0.3)
```

## Master EQ Curve

Static frequency shaping on the master bus -- shapes the overall tonal balance to help mixes translate across different playback systems. Unlike the multiband compressor (which responds dynamically), the EQ curve applies a fixed boost or cut in each band.

The curve operates on 3 bands:

| Band | Type | Center/Corner | Purpose |
|------|------|---------------|---------|
| Low | Low shelf | 120 Hz | Control sub-bass and bass warmth |
| Mid | Peak (bell) | 1 kHz | Shape body and vocal presence |
| High | High shelf | 6 kHz | Add or reduce brightness and air |

### Presets

Pre-tuned curves for common translation targets:

```sc
master curve car                          // reduce sub-bass, boost presence for road noise
master curve broadcast                    // EBU broadcast standard — flat, controlled
master curve bright                       // high shelf boost for sparkle
master curve warm                         // gentle high rolloff, low shelf boost
master curve flat                         // bypass — all bands at 0 dB
```

| Preset | Low (120 Hz) | Mid (1 kHz) | High (6 kHz) | Use case |
|--------|-------------|-------------|-------------|----------|
| `car` | -4 dB | 0 dB | +3 dB | Road noise masks sub-bass and highs; cuts low end, boosts presence |
| `broadcast` | -2 dB | 0 dB | -1 dB | Controlled, flat for broadcast compliance |
| `bright` | 0 dB | 0 dB | +3 dB | Adds air and sparkle for headphone mixes |
| `warm` | +2 dB | 0 dB | -2 dB | Gentle warmth for acoustic and vocal material |
| `flat` | 0 dB | 0 dB | 0 dB | Bypass -- no shaping |

### Manual Per-Band Control

Specify per-band gain in dB for full control:

```sc
master curve low -4, mid 0, high 3       // cut sub-bass, boost presence
master curve low 0, mid -2, high 0       // scoop mids
master curve low -6, mid 0, high 0       // aggressive sub-bass reduction
```

Each value is in dB relative to unity. Positive values boost, negative values cut.

## Multiband Compressor

Splits audio into 3 frequency bands using 4th-order Linkwitz-Riley (LR4) crossovers and compresses each independently. LR4 crossovers provide phase-coherent summation at the crossover points -- the low band uses allpass delay compensation to maintain phase alignment across all three bands.

Each band uses frequency-dependent attack and release times tuned to the content in that range:

| Band | Frequency Range | Attack | Release | Typical Content |
|------|----------------|--------|---------|-----------------|
| Low | Below 200 Hz | 15 ms | scaled | Sub-bass, kick fundamental, bass body |
| Mid | 200 Hz -- 3 kHz | 5 ms | scaled | Vocals, guitar, snare body, melodic content |
| High | Above 3 kHz | 1 ms | scaled | Hi-hats, cymbals, vocal sibilance, air |

At amount 1.0, the multiband compressor achieves approximately 4.4 dB of crest factor reduction.

### Simple Amount Control

A single value from 0 to 1.0 controls the overall compression intensity across all bands:

```sc
master multiband 0.3                      // gentle — subtle tightening per band
master multiband 0.6                      // moderate — radio-ready feel
master multiband 1.0                      // heavy — OTT-level, every detail hyper-visible
master multiband 0                        // off — bypass entirely
master multiband off                      // also bypasses
```

The amount maps to compression parameters internally:

| Amount | Threshold | Ratio | Character |
|--------|-----------|-------|-----------|
| 0.3 | -24 dB | 1.5:1 | Gentle tightening, transparent |
| 0.5 | -18 dB | 2:1 | Noticeable control, good for translation |
| 0.8 | -12 dB | 3:1 | Aggressive, dense, forward-sounding |
| 1.0 | -9 dB | 4:1 | OTT-level, extreme detail, pumping |

### Per-Band Control

For finer tuning, specify an amount per band:

```sc
master multiband low 0.5, mid 0.3, high 0.2    // tighter low end, gentle mids and highs
master multiband low 0.2, mid 0.5, high 0.4    // controlled mids and highs, relaxed lows
```

This is useful when specific bands need different treatment -- for example, taming a boomy low end while leaving the midrange open.

## Soft Clipper

A `tanh` waveshaper that sits between the compressor and the limiter. Instead of hard clipping or brick-wall limiting, it rounds off peaks with warm saturation, adding harmonic density that helps mixes translate to noisy environments (car stereo, phone speakers, laptop).

```sc
master saturate 0.5                       // gentle warmth, subtle harmonics
master saturate 0.8                       // noticeable saturation, thicker sound
master saturate 1.0                       // heavy — audible grit and density
master saturate 0                         // off — bypass
master saturate off                       // also bypasses
```

The amount controls the drive level into the waveshaper:

| Amount | Character |
|--------|-----------|
| 0.0 | Bypass -- no saturation |
| 0.3 | Transparent warmth, barely audible but adds translation resilience |
| 0.5 | Gentle saturation, pleasant harmonic density |
| 0.8 | Audible warmth, thicker transients |
| 1.0 | Heavy saturation, noticeable grit and compression |

The soft clipper is particularly effective for mixes that will be played in noisy environments. The added harmonics fill in spectral gaps that environmental noise would otherwise mask, keeping melodies and vocals intelligible under road noise or crowd chatter.

## Configuring Ceiling

Set the brick-wall limiter ceiling:

```sc
master ceiling -1.0                // set limiter ceiling to -1.0 dBFS (default: -0.3)
```

## Master Gain

Reduce the overall level before the limiter -- useful for dense mixes:

```sc
master gain -6                     // reduce overall level by 6 dB
```

## LUFS Measurement

Every `render` prints integrated loudness (LUFS, per ITU-R BS.1770) and true peak:

```
$ sound-cabinet render examples/lofi-afternoon.sc -o lofi.wav
  Integrated loudness: -15.6 LUFS
  True peak: -0.2 dBFS
Rendered to lofi.wav
```

LUFS (Loudness Units relative to Full Scale) measures perceived loudness over the entire file, weighted to match human hearing sensitivity. It is the industry standard for loudness normalization.

## Loudness Normalization

Use `--lufs` to auto-normalize to a target loudness:

```bash
sound-cabinet render track.sc -o track.wav --lufs -14
```

The normalizer applies gain after rendering to hit the target. If the resulting peak would exceed -0.1 dBFS, it warns about clipping risk.

### Platform Targets

| Platform | Target |
|---|---|
| Spotify | -14 LUFS |
| Apple Music | -16 LUFS |
| YouTube | -14 LUFS |
| Broadcast (EBU R128) | -23 LUFS |

## Realtime A/B Master Toggle & Gain Reduction

When using `play` or `piano` mode, you can instantly bypass the entire master bus chain by pressing `m` or `\`.

```bash
# In play or piano mode
m  or  \  : Toggle master bus bypass
```

**Auto-Volume Matching:** Bypassing automatically matches the RMS volume of the dry signal to the wet signal in real-time. This ensures that you aren't fooled by "louder is better" and can accurately judge the sonic improvements of your chain.

**Gain Reduction Metering:** The terminal output provides a live `[ GR -2.4 dB ]` meter, showing the total instantaneous gain reduction applied by the master compressor, multiband compressor, and limiter combined. This helps you dial in thresholds and ratios accurately.

## Test Master Command

The `test-master` command runs automated A/B testing of your master bus configuration. It renders the score with and without the master bus processing and reports the differences in loudness, crest factor, and frequency balance:

```bash
sound-cabinet test-master song.sc
```

Use this to verify that your master bus is actually improving the mix rather than just making it louder. The command compares the processed and bypassed versions and reports whether the master chain is achieving its intended effect.

## CLI Overrides

CLI flags override any `master` directives set in the score file:

```bash
# Set compression amount
sound-cabinet render track.sc -o track.wav --compress 2.0

# Explicit compression parameters
sound-cabinet render track.sc -o track.wav --compress -18,2,0.05,0.2

# Set limiter ceiling
sound-cabinet render track.sc -o track.wav --ceiling -1.0

# Bypass compression entirely, normalize to LUFS target
sound-cabinet render track.sc -o track.wav --compress 0 --lufs -14
```
