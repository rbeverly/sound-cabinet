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

def extract_pedal_windows(track):
    """Extract sustain pedal (CC64) on/off windows as [(down_tick, up_tick), ...]."""
    windows = []
    pedal_down_tick = None
    tick = 0

    for msg in track:
        tick += msg.time
        if msg.type == 'control_change' and msg.control == 64:
            if msg.value >= 64 and pedal_down_tick is None:
                pedal_down_tick = tick
            elif msg.value < 64 and pedal_down_tick is not None:
                windows.append((pedal_down_tick, tick))
                pedal_down_tick = None

    return windows


def apply_sustain_pedal(notes, pedal_windows):
    """Extend note durations based on sustain pedal windows.

    When a note's key-release falls during a pedal-down window, its duration
    is extended to the pedal-up moment. This matches real piano behavior where
    the sustain pedal holds dampers off the strings.
    """
    if not pedal_windows:
        return notes

    sustained = []
    for start_tick, end_tick, midi_note, vel in notes:
        new_end = end_tick
        for pedal_down, pedal_up in pedal_windows:
            # Note ends while pedal is down — extend to pedal up
            if pedal_down <= end_tick <= pedal_up:
                new_end = max(new_end, pedal_up)
                break
            # Note starts before pedal and ends after pedal down —
            # also extend (pedal caught a ringing note)
            if start_tick < pedal_down < end_tick:
                new_end = max(new_end, pedal_up)
                # Don't break — check further windows in case of
                # overlapping pedal presses
        sustained.append((start_tick, new_end, midi_note, vel))

    return sorted(sustained)


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


# ── Chord detection ───────────────────────────────────────────────

# Chord formulas: pitch class intervals from root → chord name
# Ordered longest first for matching priority
CHORD_FORMULAS = [
    (frozenset([0, 4, 7, 11, 14]), "maj9"),
    (frozenset([0, 3, 7, 10, 14]), "m9"),
    (frozenset([0, 4, 7, 10, 14]), "dom9"),
    (frozenset([0, 4, 7, 11]),     "maj7"),
    (frozenset([0, 3, 7, 10]),     "m7"),
    (frozenset([0, 4, 7, 10]),     "dom7"),
    (frozenset([0, 3, 6, 9]),      "dim7"),
    (frozenset([0, 4, 8, 10]),     "aug7"),
    (frozenset([0, 2, 7]),         "sus2"),
    (frozenset([0, 5, 7]),         "sus4"),
    (frozenset([0, 4, 7]),         "maj"),
    (frozenset([0, 3, 7]),         "m"),
    (frozenset([0, 3, 6]),         "dim"),
    (frozenset([0, 4, 8]),         "aug"),
]

PITCH_CLASS_NAMES = ['C', 'Db', 'D', 'Eb', 'E', 'F', 'Gb', 'G', 'Ab', 'A', 'Bb', 'B']


def identify_chord(midi_notes):
    """Try to identify a chord from a set of MIDI note numbers.

    Returns a chord name string (e.g., "Fm", "Cmaj7") or None if no match.
    Only matches if 3+ unique pitch classes are present.
    """
    if len(midi_notes) < 3:
        return None

    pitch_classes = sorted(set(n % 12 for n in midi_notes))
    if len(pitch_classes) < 3:
        return None

    # Try each pitch class as the potential root
    for root_pc in pitch_classes:
        intervals = frozenset((pc - root_pc) % 12 for pc in pitch_classes)
        for formula, quality in CHORD_FORMULAS:
            if intervals == formula:
                root_name = PITCH_CLASS_NAMES[root_pc]
                return f"{root_name}{quality}"

    return None


def detect_chords_in_pattern(pattern_notes, beat_tolerance=0.05, dur_tolerance=0.2):
    """Group simultaneous notes and detect chords. Returns list of (beat, chord_name) tuples."""
    if not pattern_notes:
        return []

    # Group notes by beat (within tolerance)
    groups = []
    current_group = [pattern_notes[0]]

    for note in pattern_notes[1:]:
        if abs(note[0] - current_group[0][0]) <= beat_tolerance:
            current_group.append(note)
        else:
            groups.append(current_group)
            current_group = [note]
    groups.append(current_group)

    chords = []
    for group in groups:
        if len(group) < 3:
            continue

        # Extract MIDI note numbers from note names
        midi_notes = []
        for _, _, _, _, note_name in group:
            midi = note_name_to_midi(note_name)
            if midi is not None:
                midi_notes.append(midi)

        if len(midi_notes) < 3:
            continue

        chord_name = identify_chord(midi_notes)
        if chord_name:
            beat = round(group[0][0], 2)
            chords.append((beat, chord_name, len(group)))

    return chords


def note_name_to_midi(name):
    """Convert a note name like 'C4', 'Ab3' to a MIDI number."""
    pc_map = {'C': 0, 'D': 2, 'E': 4, 'F': 5, 'G': 7, 'A': 9, 'B': 11}
    if not name or name[0] not in pc_map:
        return None
    pc = pc_map[name[0]]
    rest = name[1:]
    if rest.startswith('b'):
        pc -= 1
        rest = rest[1:]
    elif rest.startswith('#') or rest.startswith('s'):
        pc += 1
        rest = rest[1:]
    try:
        octave = int(rest)
    except ValueError:
        return None
    return (octave + 1) * 12 + pc


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
    pedal_windows=None,
    simplify_chords=False,
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

    # Emit sustain pedal commands (engine-level pedal)
    beats_per_second = bpm / 60.0
    if pedal_windows:
        for down_tick, up_tick in pedal_windows:
            t_down = tick_to_seconds(down_tick, tpb, tempo_map)
            t_up = tick_to_seconds(up_tick, tpb, tempo_map)
            if t_down > max_seconds:
                break
            beat_down = round(t_down * beats_per_second, 2)
            beat_up = round(min(t_up, max_seconds) * beats_per_second, 2)
            lines.append(f"pedal down at {beat_down}")
            lines.append(f"pedal up at {beat_up}")
        lines.append(f"")

    # Filter notes to time range and convert to beat-based timing.
    # CRITICAL: use the tempo map to get real-time positions, then
    # convert to output beats. This respects tempo changes in the MIDI.
    beat_notes = []  # (beat, voice_name, velocity, duration_beats, note_name)

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

        # Detect chords for annotation/simplification
        chord_annotations = detect_chords_in_pattern(pattern_notes)
        chord_beats = {c[0]: (c[1], c[2]) for c in chord_annotations}  # beat -> (name, note_count)

        # Build set of beats that are fully collapsed (all notes in chord)
        collapsed_beats = set()
        if simplify_chords:
            for ann_beat, (chord_name, note_count) in chord_beats.items():
                # Count notes at this beat
                group = [n for n in pattern_notes if abs(round(n[0], 2) - ann_beat) < 0.01]
                if len(group) == note_count:
                    # Check velocity uniformity (max 30% range)
                    vels = [n[2] for n in group]
                    vel_range = (max(vels) - min(vels)) / 127.0
                    # Check duration uniformity (within 20%)
                    durs = [n[3] for n in group]
                    dur_range = (max(durs) - min(durs)) / max(durs) if max(durs) > 0 else 0
                    if vel_range <= 0.3 and dur_range <= 0.2:
                        collapsed_beats.add(ann_beat)

        prev_beat = None
        for local_beat, voice, vel, dur, note in pattern_notes:
            b = round(local_beat, 2)
            d = round(dur, 2)
            sr = swell_release

            # If this beat is collapsed into a chord, emit one line and skip the rest
            if b in collapsed_beats:
                if b != prev_beat:
                    chord_name = chord_beats[b][0]
                    # Average velocity of the group
                    group = [n for n in pattern_notes if abs(round(n[0], 2) - b) < 0.01]
                    avg_vel = round(sum(n[2] for n in group) / (len(group) * 127.0), 2)
                    if instrument_name:
                        play_expr = f"{instrument_name}({chord_name})"
                    else:
                        play_expr = f"chord({chord_name})"
                    lines.append(f"  // {chord_name}")
                    lines.append(
                        f"  at {b} play {avg_vel} * {play_expr} >> swell(0.0, {sr}) for {d} beats"
                    )
                prev_beat = b
                continue

            # Non-collapsed: emit individual note with chord comment
            if b in chord_beats and b != prev_beat:
                lines.append(f"  // {chord_beats[b][0]}")
            prev_beat = b

            vel_gain = round(vel / 127.0, 2)

            if instrument_name:
                play_expr = f"{instrument_name}({note})"
            else:
                play_expr = voice

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
    parser.add_argument("--pedal", action="store_true", help="Apply sustain pedal (CC64) data to extend note durations")
    parser.add_argument("--simplify-chords", action="store_true", help="Collapse simultaneous notes into chord notation where lossless")
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

    pedal_windows_for_sc = None
    if args.pedal:
        pedal_windows = extract_pedal_windows(track)
        if pedal_windows:
            # Pass pedal windows to SC generation (engine handles sustain)
            # Don't extend note durations — the engine extends events at render time
            pedal_windows_for_sc = pedal_windows
            print(f"Found {len(pedal_windows)} sustain pedal windows (engine pedal)")

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
            pedal_windows=pedal_windows_for_sc,
            simplify_chords=args.simplify_chords,
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
