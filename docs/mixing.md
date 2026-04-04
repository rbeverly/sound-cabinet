[← Back to README](../README.md)

# Mixing & Diagnostics

Tools for understanding and balancing the levels in your mix.

## Profile Command

Render a score and report per-voice levels so you can spot balance issues without guessing:

```bash
sound-cabinet profile song.sc
```

Output:

```
  Voice                       RMS       Peak   Relative  Status
  -----                       ---       ----   --------  ------
  kick                   -17.2 dB    -3.4 dB    +0.0 dB  Loudest
  hat                    -19.3 dB    -1.8 dB    -2.1 dB  Loudest
  bass                   -52.1 dB   -38.0 dB   -35.0 dB  INAUDIBLE
  mel                    -33.2 dB   -15.5 dB   -16.0 dB  Quiet
```

The columns:

- **RMS** -- average loudness of the voice over the entire render
- **Peak** -- highest instantaneous sample level
- **Relative** -- how far each voice is from the loudest (the most useful number for mixing decisions)
- **Status** -- flags like `Loudest`, `Quiet`, or `INAUDIBLE`

If a voice is marked INAUDIBLE, it's effectively not in the mix -- don't spend time tweaking its sound until you fix its level.

`render` also prints a voice level summary automatically after every render.

## Solo Flag

Mute everything except one voice (or a few) to hear it in isolation:

```bash
sound-cabinet play song.sc --solo bass
sound-cabinet play song.sc --solo bass,melody
sound-cabinet render song.sc -o bass-check.wav --solo bass
```

Works with both `play` and `render`.

## Live VU Meters

Show real-time per-voice level bars during playback:

```bash
sound-cabinet play song.sc --vu
```

The display uses color coding to indicate level:

- Normal levels show in standard color
- Voices flagged as `(quiet)` need their gain raised
- Voices flagged as `(clip!)` are exceeding safe levels and need their gain reduced

Peak hold markers show the highest level reached, with gradual decay so you can see transient peaks.

## Freeze — Expand and Inspect

The `freeze` command expands all patterns, sections, pick/shuffle, humanize, and swing into a flat list of absolute `PlayAt` events. Useful for debugging timing issues and seeing exactly what the engine will play:

```bash
# Print expanded score to terminal
sound-cabinet freeze song.sc

# Save to file with deterministic seed
sound-cabinet freeze song.sc --seed 42 -o frozen.sc

# Generate multiple variations
sound-cabinet freeze song.sc --seed 99 -o variation2.sc
```

Each event includes comments showing its source pattern and voice label:

```sc
at 8 play kick for 0.5 beats  // beat, voice:kick
at 0 play warm_pad >> swell(2, 2) for 8 beats  // pad_bed, voice:warm_pad
```

The frozen output is valid `.sc` — you can `sound-cabinet play frozen.sc` and it sounds identical to the original.

## Sub-bass Fold-up

A playback-only monitoring mode that shifts sub-bass content up by 1 octave so you can hear it on headphones or small speakers. Sub-bass below ~80 Hz is largely inaudible on headphones but can rattle car doors and subwoofers -- fold-up makes it audible without needing a subwoofer to check:

```bash
sound-cabinet play song.sc --subfold
```

With `--subfold` enabled, everything below ~80 Hz is pitch-shifted up by one octave and mixed in as a quiet monitoring layer. A 40 Hz door-rattler becomes a clearly audible 160 Hz tone. This lets you detect dangerous sub-bass buildup before getting in the car.

Fold-up is playback-only -- it never affects rendered output from `render`. Use it as a diagnostic: if you hear unexpected low-frequency content during fold-up playback, add a `highpass(40)` to the offending voice or reduce its low-frequency gain.

## Environment Simulation

A monitoring-only mode that mixes calibrated environmental noise into playback to test how a mix translates to real-world listening conditions. The noise never touches the rendered file -- it is purely a diagnostic tool:

```bash
sound-cabinet play song.sc --env car       # highway road noise + cabin resonance
sound-cabinet play song.sc --env cafe      # coffee shop chatter and ambient noise
sound-cabinet play song.sc --env subway    # heavy broadband transit noise
```

### Noise Profiles

| Profile | Character | What It Tests |
|---------|-----------|---------------|
| `car` | Low-frequency road rumble with cabin resonance peaks | Whether sub-bass is excessive and whether melodies/vocals survive road noise |
| `cafe` | Mid-frequency crowd chatter and clinking | Whether midrange elements (vocals, leads) cut through conversational noise |
| `subway` | Heavy broadband noise across all frequencies | Worst-case translation -- if it works here, it works everywhere |

Noise profiles are calibrated from real-world measurements at typical listening levels.

If the melody disappears under simulated road noise, the mix needs more harmonic content in the midrange (try the `excite()` effect) or wider frequency spread. If the bass is overwhelming under `--env car`, reduce sub-bass with `master curve car` or add a highpass to the bass voice.

Environment simulation pairs well with `master curve` presets -- use `--env car` to hear the problem, then apply `master curve car` to fix it.

## Frequency-band Profile

The `profile` command includes a second table showing per-voice energy across 4 frequency bands, with warning flags for common problems:

```bash
sound-cabinet profile song.sc
```

Output includes the standard level table plus a frequency-band breakdown:

```
  Voice          Sub(<80)   Low(80-300)  Mid(300-3k)  High(3k+)   Status
  -----          --------   ----------   ----------   ---------   ------
  kick           -8.2 dB    -14.1 dB     -22.0 dB     -40.1 dB   ⚠ Sub-heavy
  bass           -12.1 dB   -16.3 dB     -28.4 dB     -45.0 dB   ⚠ Sub-heavy
  melody         -60.0 dB   -38.2 dB     -18.4 dB     -24.1 dB   OK
  pad            -55.0 dB   -20.3 dB     -16.1 dB     -38.5 dB   OK
  lead           -60.0 dB   -42.0 dB     -30.2 dB     -35.8 dB   ⚠ No presence
```

The 4 bands:

| Band | Range | Typical Content |
|------|-------|-----------------|
| Sub | Below 80 Hz | Sub-bass rumble, kick sub-harmonics |
| Low | 80 -- 300 Hz | Bass body, kick punch, low male vocal |
| Mid | 300 Hz -- 3 kHz | Vocals, guitar, snare, melodic content |
| High | Above 3 kHz | Hi-hats, cymbals, sibilance, air, sparkle |

### Warning Flags

- **Sub-heavy** -- the voice has significant energy below 80 Hz. This may cause problems on car stereos and subwoofers. Consider adding `highpass(40)` or reducing the voice's low-frequency content.
- **No presence** -- the voice has little energy in the mid and high bands relative to its low-frequency content. It may disappear in noisy environments. Consider using `excite()` to add high-frequency harmonics, or EQ to boost the 2-5 kHz range.

## Test Master Command

The `test-master` command runs automated A/B testing of your master bus configuration. It renders the score twice -- once with the master bus active and once bypassed -- and compares the results:

```bash
sound-cabinet test-master song.sc
```

Use this to verify that your master bus chain is actually improving the mix. The command reports differences in loudness, crest factor, and frequency balance between the processed and bypassed versions, so you can see whether each stage of the chain is having its intended effect.

Typical workflow:

1. Set up your master chain in the score (`master chain`, `master compress`, `master saturate`, etc.)
2. Run `sound-cabinet test-master song.sc` to see the before/after comparison
3. Adjust parameters and re-run until the numbers match your intent
4. Use `--env car` or `--env subway` alongside `test-master` to check translation

See [Master Bus & Loudness](master-bus.md#test-master-command) for more detail on the master bus chain.

## Voice Level Summary on Render

Every `render` command automatically prints a per-voice level summary after completing. This gives the same information as `profile` without a separate command.

## Volume Normalization

Different instruments produce different output levels depending on their synthesis chain. `normalize` levels them to a consistent volume:

```sc
instrument bass = sine(freq) >> lowpass(freq * 3, 0.5) >> decay(12)
instrument piano = saw(freq) >> lowpass(freq * 4, 0.7) >> decay(8)

normalize bass 0.5
normalize piano 0.5
```

The target is on a 0.0-1.0 scale where 1.0 = full scale (0 dBFS) and 0.5 = comfortable level (-6 dB). The engine renders short test tones at multiple frequencies (C2 through C6) through the instrument, measures the average RMS, and applies a gain correction.

See [Expressions & Effects Reference](expressions.md) for full details on the `normalize` directive.

## Common Mixing Problems

### Inaudible Voices

If a voice's gain multiplier is too low, it's effectively silent. In Sound Cabinet, gain is linear -- the difference between `* 0.5` and `* 0.01` is much larger than it looks:

| Linear Gain | dB Level | Perception |
|-------------|----------|------------|
| `* 1.0` | 0 dB | Full volume |
| `* 0.5` | -6 dB | Noticeably quieter |
| `* 0.25` | -12 dB | Half as loud (perceived) |
| `* 0.1` | -20 dB | Quiet |
| `* 0.01` | -40 dB | Nearly inaudible |
| `* 0.001` | -60 dB | Silent for practical purposes |

Use `sound-cabinet profile song.sc` to check. If a voice shows up as INAUDIBLE, raise its gain. Or use `normalize` to auto-level instruments to a consistent volume.

### Clipping

If the VU meters show `(clip!)` or the peak level in profile output is close to 0 dB, you have clipping. Solutions:

- Reduce the gain on the offending voice
- Use `master gain -3` to pull down the overall level
- Use `normalize` to bring instruments to a consistent, safe level
- The master bus limiter catches peaks at -0.3 dBFS, but it's better to fix the source

### Voices That Disappear in the Mix

A voice that sounds fine solo but disappears in context is usually being masked by another voice in the same frequency range. Solutions:

- Use EQ to carve space: cut competing frequencies on one voice, boost on the other (see [Expressions & Effects Reference](expressions.md) for parametric EQ)
- Use `--solo voice1,voice2` to listen to just the competing pair
- Check `profile` output -- if two voices have similar RMS levels but one is inaudible in context, frequency masking is likely the problem
