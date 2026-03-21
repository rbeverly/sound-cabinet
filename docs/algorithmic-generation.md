# Algorithmic Phrase Generation

Design document for a pattern-driven music generation system that composes
phrases from layered, reusable building blocks.

## Motivation

Musical composer's block is real. Instead of asking "what note comes
next?", we encode the *shapes* of musical phrases as reusable pattern files,
then combine and resolve them against a key, chord progression, and instrument
range to produce concrete `.sc` output.  Because patterns are separated from
pitch material, a small library of patterns produces a large number of
distinct phrases through combinatorics --- and every pattern added is
permanently useful.

## Layered Decomposition

A phrase is assembled from independent layers that are each individually
constrained enough to produce good results:

### 1. Rhythm

Where in the bar notes land, and how long they sound.  Fully separable from
pitch --- a bossa nova bass rhythm works in any key.

```yaml
# Example: swung eighths
rhythm:
  time: 4/4
  hits: [1/8, 1/8, ~, 1/8, 1/8, ~, 1/8, 1/8]
```

`~` denotes a rest.  Dotted values use standard notation: `1/8.` is a dotted
eighth.  Tied notes that cross bar lines use `+`: `1/4+1/8` is a quarter tied
to an eighth.

### 2. Contour (interval pattern)

Relative melodic motion, not absolute pitches.  Describes the *shape* of a
phrase without committing to a key.

```yaml
contour: [root, step_up, step_up, leap_down_3, step_up, hold, step_down, root]
```

Vocabulary of contour tokens:

| Token | Meaning |
|-------|---------|
| `root` | Scale degree 1 (or chord root, context-dependent) |
| `hold` | Repeat previous pitch |
| `step_up` / `step_down` | Move one diatonic step |
| `half_up` / `half_down` | Move one chromatic semitone |
| `leap_up_N` / `leap_down_N` | Jump N diatonic steps |
| `chord_low` / `chord_mid` / `chord_high` | Chord tones, ordered by pitch |
| `approach` | Chromatic half-step into next bar's target |
| `neighbor_up` / `neighbor_down` | Step away and return (ornamental) |
| `passing` | Diatonic step connecting two chord tones |

### 3. Emphasis (dynamics)

Which notes get accented.  Maps to gain values in `.sc` output.

```yaml
emphasis: [strong, weak, medium, weak, strong, weak, medium, weak]
```

Levels: `strong` (1.0), `medium` (0.7), `weak` (0.4), `ghost` (0.2).
When omitted, defaults to standard metric emphasis for the time signature.

### 4. Scale / mode mapping

Pins abstract contour to a concrete pitch set.  `step_up` means a whole step
in major, a half step between degrees 3-4 in natural minor, etc.

Specified at generation time, not in the pattern file:

```
key: D
mode: dorian
```

### 5. Harmonic context

Which chord is active during each beat or bar.  Constrains which scale degrees
sound consonant.  A `root` token resolves to the chord root, not necessarily
the key root.

```
progression: [Dm7, G7, Cmaj7, Cmaj7]
```

### 6. Range constraint

Instrument pitch boundaries.  The generator keeps all output within range,
inverting intervals when needed:

```
range: [C2, G3]    # bass register
```

## Pattern Files

Pattern files are YAML documents stored in a patterns library.  Each file
describes one reusable musical gesture.

```yaml
# patterns/bass/walking-jazz.yaml
name: Walking Jazz Bass
type: bass
tags: [jazz, walking, quarter-note]
time: 4/4

rhythm:
  hits: [1/4, 1/4, 1/4, 1/4]

contour: [root, step_up, step_up, approach]

emphasis: [strong, weak, weak, medium]

notes: |
  Classic jazz walking pattern.  The 'approach' token on beat 4
  creates a chromatic leading tone into the next bar's root,
  providing forward motion.  Works over any chord progression.
```

```yaml
# patterns/bass/root-fifth-country.yaml
name: Root-Fifth Country
type: bass
tags: [country, folk, alternating]
time: 4/4

rhythm:
  hits: [1/4, ~, 1/4, ~]

contour: [root, ~, leap_up_4, ~]    # root then fifth

emphasis: [strong, ~, medium, ~]
```

```yaml
# patterns/ornament/mordent-upper.yaml
name: Upper Mordent
type: ornament
tags: [baroque, ornament, fast]
time: any

rhythm:
  hits: [1/32, 1/32, remainder]

contour: [target, neighbor_up, target]

emphasis: [medium, ghost, strong]

notes: |
  Rapid alternation with the note above, returning to the main note.
  'remainder' means this ornament borrows time from the host note
  rather than adding duration.  Applied on top of an existing phrase.
```

```yaml
# patterns/melody/question-phrase.yaml
name: Question Phrase (ascending)
type: melody
tags: [phrase, tension, ascending]
time: 4/4

rhythm:
  hits: [1/8, 1/8, 1/4, 1/8, 1/8, 1/4]

contour: [root, step_up, leap_up_3, step_down, step_up, step_up]

emphasis: [medium, weak, strong, weak, medium, strong]

notes: |
  Rising phrase that ends above where it started, creating tension
  and expectation of a resolving 'answer' phrase.  Pair with
  answer-phrase-descending for antecedent-consequent structure.
```

```yaml
# patterns/accomp/alberti-bass.yaml
name: Alberti Bass
type: accompaniment
tags: [classical, keyboard, arpeggiated]
time: 4/4

rhythm:
  hits: [1/8, 1/8, 1/8, 1/8, 1/8, 1/8, 1/8, 1/8]

contour: [chord_low, chord_high, chord_mid, chord_high,
          chord_low, chord_high, chord_mid, chord_high]

emphasis: [strong, weak, medium, weak, strong, weak, medium, weak]
```

## Ornamentation as a Separate Layer

Ornaments are not melody --- they are commentary on melody.  A trill doesn't
change the melodic narrative; it decorates a note that's already there.  This
suggests ornaments should be applied *after* the base phrase is resolved,
not mixed into the phrase definition.

The generation pipeline:

1. Resolve base phrase (rhythm + contour + key + chords -> concrete notes)
2. Apply ornamentation pass with an ornamentation level

Ornamentation level controls density:

| Level | Behavior |
|-------|----------|
| 0 | None --- play the phrase as written |
| 1 | Sparse --- occasional mordents on strong beats |
| 2 | Moderate --- turns, grace notes on longer notes |
| 3 | Florid --- baroque-level decoration, fills between notes |

Each ornament pattern specifies where it can attach (strong beats, long notes,
phrase endings, etc.) and the generator probabilistically applies them based on
the level.

## Generator Pipeline

```
Pattern file(s)
     |
     v
[Rhythm layer] ----+
[Contour layer] ---+--> Resolver ---> Concrete pitches + durations
[Key/mode] --------+        |
[Chord progression] --------+
[Range constraint] ---------+
                             |
                             v
                    [Ornamentation pass]
                             |
                             v
                    [Consonance check]  <-- flag or auto-fix clashes
                             |
                             v
                      .sc file output
```

### Consonance / clash detection

When generating multiple simultaneous voices, the generator checks for:

- **Parallel fifths / octaves** between voices (classical constraint, optional)
- **Dissonant intervals** on strong beats against the active chord
- **Range collisions** where two voices occupy the same register
- **Rhythmic saturation** where too many voices attack simultaneously

Clashes can be handled by:
1. Flagging for human review (default)
2. Auto-substituting the nearest consonant pitch
3. Ignoring (for deliberately dissonant styles)

## Output: .sc Score

The generator writes standard Sound Cabinet score files.  A generation run
might produce multiple named variations:

```sc
bpm 120

// --- Generated by: walking-jazz + D dorian + [Dm7 G7 Cmaj7 Cmaj7] ---

// Variation 1
pattern bas_a = 4 beats
  at 0    play bass D2  for 1 beats
  at 1    play bass E2  for 1 beats
  at 2    play bass F#2 for 1 beats
  at 2.75 play bass C#3 for 0.25 beats   // approach to Dm root

// Variation 2 (same pattern, different contour seed)
pattern bas_b = 4 beats
  at 0    play bass D2  for 1 beats
  at 1    play bass F2  for 1 beats
  at 2    play bass A2  for 1 beats
  at 2.75 play bass Db2 for 0.25 beats
```

The human (or their LLM collaborator) picks the best variations:

> "Let's use bas_a, mel_c, mel_f for the verse, and bas_b, mel_j for the
> chorus."

Or for generative / streaming music, use `pick()`:

```sc
section groove = 16 beats
  repeat pick(bas_a, bas_b, bas_c) every 4 beats
  repeat pick(mel_a, mel_c, mel_f, mel_j) every 4 beats
```

## Workflow Integration

### CLI

```bash
# Generate 5 bass line variations over a chord progression
sound-cabinet generate \
  --pattern patterns/bass/walking-jazz.yaml \
  --key D --mode dorian \
  --chords "Dm7 G7 Cmaj7 Cmaj7" \
  --voice bass \
  --variations 5 \
  -o generated/bass-lines.sc

# Generate with ornamentation
sound-cabinet generate \
  --pattern patterns/melody/question-phrase.yaml \
  --key C --mode major \
  --chords "C Am F G" \
  --voice piano \
  --ornament 2 \
  --ornament-patterns patterns/ornament/*.yaml \
  --variations 10 \
  -o generated/melodies.sc
```

### As a compositional aid

The intended workflow is not "generate a finished song" but:

1. Pick patterns and parameters that fit the mood
2. Generate a batch of variations
3. Listen through them (`sound-cabinet play generated/bass-lines.sc`)
4. Cherry-pick the ones that work
5. Assemble into a composition, adding human-written parts as needed
6. Iterate --- swap variations, try different ornament levels, adjust chords

## Pattern Library Organization

```
patterns/
  bass/
    walking-jazz.yaml
    root-fifth-country.yaml
    syncopated-funk.yaml
    octave-pulse.yaml
    pedal-tone.yaml
  melody/
    question-phrase.yaml
    answer-phrase.yaml
    arch-contour.yaml
    sequence-ascending.yaml
    call-response.yaml
  accomp/
    alberti-bass.yaml
    block-chords.yaml
    oom-pah.yaml
    tremolo-roll.yaml
  rhythm/
    four-on-floor.yaml
    boom-bap.yaml
    bossa-nova.yaml
    shuffle.yaml
  ornament/
    mordent-upper.yaml
    mordent-lower.yaml
    turn.yaml
    trill.yaml
    grace-note.yaml
    passing-tone.yaml
    appoggiatura.yaml
```

Patterns are tagged for searchability.  A composer can browse by type, genre,
or mood.  Community contributions could grow the library over time.

## Design Decisions

### Multi-bar arcs are necessary

Single-bar patterns are useful building blocks, but a melody assembled only
from short fragments will sound fragmented --- like building paragraphs from
randomly selected sentence fragments.  The coherence of a longer phrase comes
from a thread running through it: tension building over bars 1--3, resolution
in bar 4.  The library should support both:

- **Single-bar patterns** for bass lines, accompaniment, and rhythmic cells
  where repetition is the point
- **Multi-bar arc patterns** (2, 4, 8 bars) for melodies and any voice where
  the phrase *is* the musical thought

Arc patterns use the same contour vocabulary but over a longer sequence,
and may include structural markers like `phrase_peak` or `cadence` to indicate
the shape's dramatic high point and resolution.

### Rhythm generation: worth experimenting

Rhythm is inherently repetitive, which works in our favor.  A randomly
generated 1-bar rhythm pattern has a decent chance of sounding intentional
precisely *because* it will repeat --- repetition creates the perception of
rhythm.  This is unlike melody, where a random sequence of pitches sounds
aimless no matter how many times you repeat it.

Worth trying: a rhythm generator constrained by density (how many hits per
bar), subdivision (eighths vs. sixteenths), and style rules (e.g., "kick on
1 and 3, never two hits closer than a sixteenth").  If it produces usable
results, those could feed back into the rhythm pattern library --- the
generator bootstraps its own library.

### pick() compatibility

Variations generated for the same slot (same key, same chord progression,
same bar count) are inherently `pick()`-compatible.  As long as they share
timing and harmonic context, they'll mesh at boundaries --- similar to how
loop selectors in DAWs work.  Sound Cabinet's `pick()` already handles
random selection; the generator just needs to output sets that are tagged
as belonging to the same harmonic/rhythmic slot.

## Open Questions

- **Probability weights on contour tokens**: Rather than fixed sequences,
  allow weighted alternatives: `[root, step_up|leap_up_3(0.3), ...]`.
  More expressive but harder to reason about.

- **Human-in-the-loop rating**: After a composer picks favorites, could
  those choices feed back into pattern weights?  "I always pick variations
  with stepwise motion" -> increase step probability in future generation.
