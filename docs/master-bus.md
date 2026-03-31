[← Back to README](../README.md)

# Master Bus & Loudness

Every render in Sound Cabinet passes through an automatic master bus chain before output. This applies to all modes: `render`, `play`, `watch`, `piano`, and `stream`.

## Master Bus Chain

The chain runs in this order:

1. **Highpass at 30 Hz** -- removes inaudible sub-bass that eats headroom (Butterworth 2nd-order)
2. **Lowpass at 18 kHz** -- removes ultrasonic content from aliasing and filter resonance (Butterworth 2nd-order)
3. **RMS compressor** -- reduces crest factor (the gap between peak transients and sustained content), raising perceived loudness
4. **Brick-wall limiter at -0.3 dBFS** -- prevents peaks from hitting 0 dBFS, with 5ms lookahead for clean transient handling

The master bandpass reclaims headroom stolen by inaudible frequencies, the compressor tightens dynamics, and the limiter catches peaks.

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
