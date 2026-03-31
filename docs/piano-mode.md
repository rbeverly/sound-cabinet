[← Back to README](../README.md)

# Piano Mode & MIDI

Piano mode lets you play any instrument or custom waveform live with your keyboard or a MIDI controller.

## Basic Usage

```bash
sound-cabinet piano examples/voices/concerto2-kit.sc piano
sound-cabinet piano examples/wave-test.sc spike
sound-cabinet piano examples/voices/lofi-kit.sc mel
```

The first argument is a score file (loads its instrument/voice/fx/wave definitions). The optional second argument is the instrument or wave name to play. Without it, a default sine+decay tone is used.

### Keyboard Layout

The QWERTY keyboard maps two chromatic octaves (C3-C5) across your layout -- the same mapping as GarageBand. This gives you enough range for melody and chord work without an external controller.

### Instrument Selection

Any instrument, voice, or custom waveform defined in the loaded score file can be played. Pass the name as the second argument:

```bash
sound-cabinet piano voices/kit.sc rhodes
sound-cabinet piano voices/kit.sc nylon_guitar
sound-cabinet piano examples/wave-test.sc plateau
```

## MIDI Keyboard Support

A MIDI keyboard provides the full range with velocity sensitivity. If a MIDI device is connected, it's auto-detected -- both keyboard and MIDI work simultaneously.

```bash
# Auto-detect MIDI (connects to first available device)
sound-cabinet piano voices/kit.sc piano --midi

# Specific MIDI port by index
sound-cabinet piano voices/kit.sc piano --midi 0
```

## Velocity Curves

Control how MIDI velocity maps to volume with the `--velocity` flag:

```bash
sound-cabinet piano voices/kit.sc piano --midi --velocity supersoft
```

| Curve | Description |
|-------|-------------|
| `linear` | 1:1 velocity mapping (default) |
| `soft` | Light touches produce more volume |
| `supersoft` | Very light touches still produce full volume |
| `hard` | Requires harder presses for volume |
| `full` | All notes play at full velocity regardless of input |

## Recording

Capture what you play as `.sc` patterns with timing and velocity.

### Controls

| Key | Action |
|-----|--------|
| F1 | Start/stop recording (with metronome click) |
| F2 | Save recording to `recorded_N.sc` |
| F3 | Discard current recording |
| Esc | Quit piano mode |

While recording, a metronome click sounds on each beat. Notes are timestamped and saved with beat offsets relative to the BPM.

### Recorded File Format

The output is a standard `.sc` file with a timestamped filename (e.g., `recorded_1.sc`). It contains:

- An `import` statement for the voice file you were playing
- A `bpm` setting matching the recording tempo
- Events with beat-relative timing (`at N play instrument(note) for M beats`)
- Pedal events if sustain was used during recording

The file can be imported directly into a score:

```sc
import recorded_1.sc
play recorded_pattern
```

## Sustain Pedal

### Keyboard

Press **F4** to toggle the sustain pedal on and off. Notes that are sounding when the pedal is down will ring until the pedal is released.

### MIDI

MIDI CC64 (the standard sustain pedal control) is automatically recognized. A physical sustain pedal connected to your MIDI keyboard works as expected.

## Note-Off / Release

Notes are released when the key is lifted (keyboard) or when a MIDI note-off message is received. With the sustain pedal down, note-off is deferred until the pedal comes up.
