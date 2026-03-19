#!/usr/bin/env python3
"""
midi2sc.py — Convert a MIDI file to Sound Cabinet .sc format.

Reads a MIDI file, extracts a specified track, and generates a .sc
composition file using instruments (freq-parametric voice templates).

Usage:
    python3 midi2sc.py input.mid --track 10 --max-time 155 --voice-kit voices/concerto2-kit.sc -o concerto2.sc
    python3 midi2sc.py input.mid --track 10 --instrument piano --voice-kit voices/concerto2-kit.sc -o concerto2.sc
"""

import argparse
import sys
from collections import defaultdict

try:
    import mido
except ImportError:
    print("Error: mido not installed. Run: pip install mido --break-system-packages")
    sys.exit(1)


# ── Note name mapping ──────────────────────────────────────────────
NOTE_NAMES = ['C', 'Db', 'D', 'Eb', 'E', 'F', 'Gb', 'G', 'Ab', 'A', 'Bb', 'B']

def midi_to_note_name(midi_num):
    """Convert MIDI note number to Sound Cabinet note name (e.g., 60 → C4)."""
    octave = (midi_num // 12) - 1
    name = NOTE_NAMES[midi_num % 12]
    return f"{name}{octave}"

def midi_to_voice_name(midi_num, prefix="p_"):
    """Convert MIDI note number to voice name (e.g., 60 → p_c4)."""
    octave = (midi_num // 12) - 1
    name = NOTE_NAMES[midi_num % 12].lower()
    return f"{prefix}{name}{octave}"


# ── MIDI parsing ───────────────────────────────────────────────────
def build_tempo_map(midi_file):
    """Extract tempo changes from track 0."""
    tempo_map = []
    tick = 0
    for msg in midi_file.tracks[0]:
        tick += msg.time
        if msg.type == 'set_tempo':
            tempo_map.append((tick, msg.tempo))
    return tempo_map

def tick_to_seconds(target_tick, tpb, tempo_map):
    """Convert MIDI tick to seconds using the tempo map."""
    current_tick = 0
    current_time = 0.0
    current_tempo = 500000  # default 120 BPM

    for map_tick, map_tempo in tempo_map:
        if map_tick > target_tick:
            break
        elapsed_ticks = map_tick - current_tick
        current_time += (elapsed_ticks / tpb) * (current_tempo / 1_000_000)
        current_tick = map_tick
        current_tempo = map_tempo

    remaining = target_tick - current_tick
    current_time += (remaining / tpb) * (current_tempo / 1_000_000)
    return current_time

def tick_to_beats(target_tick, tpb):
    """Convert MIDI tick to beat number."""
    return target_tick / tpb

def extract_notes(track):
    """Extract note events as (start_tick, end_tick, midi_note, velocity)."""
    notes = []
    active = {}  # note -> (start_tick, velocity)
    tick = 0

    for msg in track:
        tick += msg.time
        if msg.type == 'note_on' and msg.velocity > 0:
            active[msg.note] = (tick, msg.velocity)
        elif (msg.type == 'note_on' and msg.velocity == 0) or msg.type == 'note_off':
            if msg.note in active:
                start_tick, vel = active.pop(msg.note)
                notes.append((start_tick, tick, msg.note, vel))

    return sorted(notes)


# ── SC generation ──────────────────────────────────────────────────
def generate_sc(
    notes,
    tpb,
    tempo_map,
    bpm,
    max_seconds,
    voice_prefix,
    voice_kit_path,
    min_duration_beats,
    sustain_duration_beats,
    swell_release,
    pattern_size_beats,
    instrument_name=None,
):
    """Generate .sc file content from extracted MIDI notes."""

    lines = []
    lines.append(f"// Auto-generated from MIDI by midi2sc.py")
    lines.append(f"// Voice kit: {voice_kit_path}")
    lines.append(f"")
    lines.append(f"import {voice_kit_path}")
    lines.append(f"")
    lines.append(f"bpm {bpm}")
    lines.append(f"")

    # Filter notes to time range and convert to beat-based timing.
    # CRITICAL: use the tempo map to get real-time positions, then
    # convert to output beats. This respects tempo changes in the MIDI.
    beat_notes = []  # (beat, voice_name, velocity, duration_beats, note_name)
    beats_per_second = bpm / 60.0

    for start_tick, end_tick, midi_note, vel in notes:
        t_start = tick_to_seconds(start_tick, tpb, tempo_map)
        if t_start > max_seconds:
            break

        t_end = tick_to_seconds(end_tick, tpb, tempo_map)
        dur_seconds = t_end - t_start

        # Convert real time to output beats
        beat = t_start * beats_per_second
        dur_beats = dur_seconds * beats_per_second

        voice = midi_to_voice_name(midi_note, voice_prefix)
        note = midi_to_note_name(midi_note)

        # Apply sustain pedal simulation: extend short notes
        play_dur = max(dur_beats, min_duration_beats)
        if dur_beats < sustain_duration_beats:
            play_dur = sustain_duration_beats

        beat_notes.append((beat, voice, vel, play_dur, note))

    if not beat_notes:
        return "// No notes found in the specified time range.\n"

    # Determine total beats
    last_beat = max(b + d for b, _, _, d, _ in beat_notes)
    total_beats = int(last_beat) + 4  # pad with a few beats

    # Split into patterns of pattern_size_beats
    pattern_count = (total_beats + pattern_size_beats - 1) // pattern_size_beats

    # Group notes by pattern
    patterns = defaultdict(list)
    for beat, voice, vel, dur, note in beat_notes:
        pattern_idx = int(beat // pattern_size_beats)
        local_beat = beat - (pattern_idx * pattern_size_beats)
        patterns[pattern_idx].append((local_beat, voice, vel, dur, note))

    # Generate patterns
    for idx in range(pattern_count):
        if idx not in patterns:
            continue

        pattern_notes = sorted(patterns[idx])
        if not pattern_notes:
            continue

        lines.append(f"pattern part_{idx} = {pattern_size_beats} beats")

        for local_beat, voice, vel, dur, note in pattern_notes:
            # Round beat to 2 decimal places for readability
            b = round(local_beat, 2)
            d = round(dur, 2)
            sr = swell_release

            # Velocity as a gain multiplier (0.0-1.0)
            vel_gain = round(vel / 127.0, 2)

            if instrument_name:
                # Use instrument syntax: piano(C4)
                play_expr = f"{instrument_name}({note})"
            else:
                # Legacy voice syntax: p_c4
                play_expr = voice

            # Apply velocity as gain and swell for release
            lines.append(
                f"  at {b} play {vel_gain} * {play_expr} >> swell(0.0, {sr}) for {d} beats"
            )

        lines.append(f"")

    # Generate arrangement
    lines.append(f"// ── Arrangement ──")
    for idx in range(pattern_count):
        if idx in patterns:
            lines.append(f"play part_{idx}")

    lines.append(f"")
    return "\n".join(lines)


# ── Voice kit generator ────────────────────────────────────────────
def generate_voice_kit(notes, tpb, tempo_map, max_seconds, voice_prefix):
    """Generate a voice kit with all pitches used in the MIDI."""

    pitches_used = set()
    for start_tick, end_tick, midi_note, vel in notes:
        t = tick_to_seconds(start_tick, tpb, tempo_map)
        if t > max_seconds:
            break
        pitches_used.add(midi_note)

    lines = []
    lines.append(f"// Auto-generated voice kit from MIDI")
    lines.append(f"// Piano voice recipe (hammer + strings + overall decay)")
    lines.append(f"//")
    lines.append(f"// Voices use the proven recipe:")
    lines.append(f"//   HAMMER: saw >> bright lowpass >> fast decay")
    lines.append(f"//   STRINGS: saw + octave-up saw >> dark lowpass >> chorus")
    lines.append(f"//   OVERALL: decay >> reverb (high damping)")
    lines.append(f"")

    # Group by octave
    by_octave = defaultdict(list)
    for midi_note in sorted(pitches_used):
        octave = (midi_note // 12) - 1
        by_octave[octave].append(midi_note)

    # Voice parameters per octave (from our tuning sessions)
    octave_params = {
        1: {"hammer_amp": 0.55, "hammer_lp": 1200, "hammer_decay": 6, "string_amp": 1.80, "octave_amp": 0.35, "string_lp": 180, "overall_decay": 1.2, "reverb": "reverb(0.6, 0.3, 0.2)"},
        2: {"hammer_amp": 0.50, "hammer_lp": 1400, "hammer_decay": 7, "string_amp": 1.80, "octave_amp": 0.35, "string_lp": 200, "overall_decay": 1.5, "reverb": "reverb(0.6, 0.3, 0.2)"},
        3: {"hammer_amp": 0.45, "hammer_lp": 1800, "hammer_decay": 8, "string_amp": 1.80, "octave_amp": 0.35, "string_lp": "2x", "overall_decay": 2.0, "reverb": "reverb(0.6, 0.3, 0.2)"},
        4: {"hammer_amp": 0.42, "hammer_lp": 2200, "hammer_decay": 9, "string_amp": 1.50, "octave_amp": 0.35, "string_lp": 600, "overall_decay": 2.5, "reverb": "reverb(0.6, 0.6, 0.35)"},
        5: {"hammer_amp": 0.38, "hammer_lp": 2800, "hammer_decay": 10, "string_amp": 2.00, "octave_amp": 0.35, "string_lp": 600, "overall_decay": 3.0, "reverb": "reverb(0.6, 0.6, 0.4)"},
        6: {"hammer_amp": 0.33, "hammer_lp": 2800, "hammer_decay": 10, "string_amp": 2.00, "octave_amp": 0.35, "string_lp": 650, "overall_decay": 3.0, "reverb": "reverb(0.6, 0.6, 0.4)"},
        7: {"hammer_amp": 0.28, "hammer_lp": 2800, "hammer_decay": 10, "string_amp": 2.00, "octave_amp": 0.30, "string_lp": 700, "overall_decay": 3.0, "reverb": "reverb(0.6, 0.6, 0.4)"},
    }

    # Frequencies for string lowpass calculation (octave 3 uses 2x fundamental)
    FREQS = {
        'C': 16.35, 'Db': 17.32, 'D': 18.35, 'Eb': 19.45, 'E': 20.60,
        'F': 21.83, 'Gb': 23.12, 'G': 24.50, 'Ab': 25.96, 'A': 27.50,
        'Bb': 29.14, 'B': 30.87
    }

    for octave in sorted(by_octave.keys()):
        params = octave_params.get(octave, octave_params[5])  # default to octave 5 params
        lines.append(f"// ── Octave {octave} ──")

        for midi_note in by_octave[octave]:
            note_name = midi_to_note_name(midi_note)
            voice_name = midi_to_voice_name(midi_note, voice_prefix)
            note_letter = NOTE_NAMES[midi_note % 12]
            octave_up = f"{note_letter}{octave + 1}"

            # Calculate string lowpass
            base_freq = FREQS[note_letter] * (2 ** octave)
            if params["string_lp"] == "2x":
                slp = int(base_freq * 2)
            else:
                slp = params["string_lp"]

            ha = params["hammer_amp"]
            hlp = params["hammer_lp"]
            hd = params["hammer_decay"]
            sa = params["string_amp"]
            oa = params["octave_amp"]
            od = params["overall_decay"]
            rev = params["reverb"]

            lines.append(
                f"voice {voice_name} = (({ha} * saw({note_name}) >> lowpass({hlp}, 0.7) >> decay({hd})) "
                f"+ ({sa} * saw({note_name}) + {oa} * saw({octave_up})) >> lowpass({slp}, 0.6) "
                f">> chorus(0.016, 0.006, 0.1)) >> decay({od}) >> {rev}"
            )

        lines.append(f"")

    return "\n".join(lines)


# ── Main ───────────────────────────────────────────────────────────
def main():
    parser = argparse.ArgumentParser(description="Convert MIDI to Sound Cabinet .sc format")
    parser.add_argument("input", help="Input MIDI file")
    parser.add_argument("-o", "--output", help="Output .sc file", default="output.sc")
    parser.add_argument("--track", type=int, default=0, help="MIDI track number to extract")
    parser.add_argument("--max-time", type=float, default=999, help="Max time in seconds to convert")
    parser.add_argument("--bpm", type=int, default=66, help="BPM for the output .sc file")
    parser.add_argument("--voice-prefix", default="p_", help="Prefix for voice names")
    parser.add_argument("--voice-kit", default="voices/concerto2-kit.sc", help="Voice kit import path")
    parser.add_argument("--min-duration", type=float, default=0.5, help="Minimum note duration in beats")
    parser.add_argument("--sustain", type=float, default=1.5, help="Sustain pedal duration in beats (short notes extended to this)")
    parser.add_argument("--swell-release", type=float, default=0.5, help="Swell release time")
    parser.add_argument("--pattern-size", type=int, default=16, help="Pattern size in beats")
    parser.add_argument("--instrument", default=None, help="Instrument name to use (e.g., 'piano'). Uses instrument(Note) syntax instead of per-note voices.")
    parser.add_argument("--generate-kit", action="store_true", help="Generate voice kit file instead of composition")
    parser.add_argument("--kit-output", default="voices/auto-kit.sc", help="Output path for generated voice kit")
    parser.add_argument("--list-tracks", action="store_true", help="List tracks and exit")

    args = parser.parse_args()

    mid = mido.MidiFile(args.input)
    tempo_map = build_tempo_map(mid)

    if args.list_tracks:
        for i, track in enumerate(mid.tracks):
            note_count = sum(1 for msg in track if msg.type == 'note_on' and msg.velocity > 0)
            print(f"Track {i}: '{track.name}' — {note_count} note-ons")
        return

    if args.track >= len(mid.tracks):
        print(f"Error: track {args.track} doesn't exist (file has {len(mid.tracks)} tracks)")
        sys.exit(1)

    track = mid.tracks[args.track]
    notes = extract_notes(track)
    print(f"Extracted {len(notes)} notes from track {args.track}: '{track.name}'")

    if args.generate_kit:
        kit_content = generate_voice_kit(notes, mid.ticks_per_beat, tempo_map, args.max_time, args.voice_prefix)
        with open(args.kit_output, 'w') as f:
            f.write(kit_content)
        print(f"Voice kit written to {args.kit_output}")
        # Count voices
        voice_count = kit_content.count("\nvoice ")
        print(f"Generated {voice_count} voice definitions")
    else:
        sc_content = generate_sc(
            notes,
            mid.ticks_per_beat,
            tempo_map,
            args.bpm,
            args.max_time,
            args.voice_prefix,
            args.voice_kit,
            args.min_duration,
            args.sustain,
            args.swell_release,
            args.pattern_size,
            instrument_name=args.instrument,
        )
        with open(args.output, 'w') as f:
            f.write(sc_content)
        print(f"Composition written to {args.output}")
        # Count notes and patterns
        note_count = sc_content.count("  at ")
        pattern_count = sc_content.count("pattern ")
        print(f"Generated {pattern_count} patterns with {note_count} note events")


if __name__ == "__main__":
    main()
