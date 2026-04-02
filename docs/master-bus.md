[← Back to README](../README.md)

# Master Bus & Loudness

Every render in Sound Cabinet passes through an automatic master bus chain before output. This applies to all modes: `render`, `play`, `watch`, `piano`, and `stream`.

## Master Bus Chain

The chain runs in this order:

1. **Highpass at 30 Hz** -- removes inaudible sub-bass that eats headroom (Butterworth 2nd-order)
2. **Lowpass at 18 kHz** -- removes ultrasonic content from aliasing and filter resonance (Butterworth 2nd-order)
3. **EQ Curve** -- static frequency shaping for translation (presets or manual 3-band)
4. **Multiband compressor** -- per-band dynamic control across low/mid/high
5. **RMS compressor** -- reduces crest factor (the gap between peak transients and sustained content), raising perceived loudness
6. **Soft clipper** -- tanh waveshaper that catches peaks with warm saturation
7. **Brick-wall limiter at -0.3 dBFS** -- prevents peaks from hitting 0 dBFS, with 5ms lookahead for clean transient handling

```
HP 30Hz → LP 18kHz → EQ Curve → Multiband Compressor → Compressor → Soft Clipper → Limiter
```

The master bandpass reclaims headroom stolen by inaudible frequencies, the EQ curve shapes frequency balance for translation, the multiband compressor controls per-band dynamics, the compressor tightens overall dynamics, the soft clipper adds harmonic density, and the limiter catches peaks.

## Configuring Compression

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

Splits audio into 3 frequency bands and compresses each independently. This is the core tool for making mixes translate -- it brings up quiet details and tames loud transients within each band without affecting the others.

### Band Splits

| Band | Frequency Range | Typical Content |
|------|----------------|-----------------|
| Low | Below 200 Hz | Sub-bass, kick fundamental, bass body |
| Mid | 200 Hz -- 3 kHz | Vocals, guitar, snare body, melodic content |
| High | Above 3 kHz | Hi-hats, cymbals, vocal sibilance, air |

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
