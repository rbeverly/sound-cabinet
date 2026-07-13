# Tasks

Document only — do not change generation behavior. The exact YAML keys are the
serde fields in `src/generate/pattern.rs` (`PatternFile`, `MotifSpec`,
`SongFile`, `SongPart`, `DrumPattern`, `DrumVoice`); confirm against them and
against the example files before writing each section so the docs match what
the loader actually accepts.

## docs/algorithmic-generation.md — add the three undocumented formats

- [x] Add a **"Motif patterns"** section. Document the `motif` block
  (`rhythm`, `contour`, `emphasis` — same vocabularies the existing layered
  sections already describe) and that a motif pattern supplies **either** an
  explicit `structure:` list **or** a `complexity:` level (not the direct
  `rhythm`+`contour` form). List the `structure` transformation vocabulary:
  `statement`, `repeat`, `sequence_up`, `sequence_down`, `inversion`,
  `retrograde`, `augmentation`, `truncation`, `extension`, `departure`
  (and `departure_high` / `departure_low`), `return`, `resolve`, `approach`,
  `rest`. Document the three `complexity` levels (`simple`, `moderate`,
  `complex`) and the `structure` each auto-expands to — transcribe the
  mappings from `default_structure()` in `src/generate/motif.rs` (the
  authoritative source) rather than inventing them. Reference
  `patterns/motif/` (`folk-simple.yaml`, `pop-verse.yaml`, etc.) as worked
  examples.

- [x] Add a **"Song files"** section. Document the top-level `parts` map —
  each part has a `motif`, an optional `structure` or `complexity`, an
  optional per-part `chords` override (a chord string scoped to that part),
  and an optional per-part `range` — plus the `arrangement` list that names
  parts in playback order. State that `generate --pattern <song.yaml>`
  auto-detects a song file (it has a `parts` key) and renders the full
  arrangement. Reference `patterns/song/` (e.g. `verse-chorus-bridge.yaml`)
  as a worked example.

- [x] Add a **"Drum patterns"** section. Document the `voices` list — each
  voice has `voice`, `pitch`, `rhythm`, and an optional `emphasis` (and that
  drum `rhythm`/`emphasis` use the `~/4` and `~` rest forms shown in the
  examples). State that `generate --pattern <drums.yaml>` auto-detects a drum
  pattern (it has a `voices` key). Reference `patterns/drums/` (`basic-rock`,
  `boom-bap`, `bossa-nova`, `waltz`) as worked examples.

- [x] Add a short note (near the CLI/Workflow section) that
  `generate --pattern` accepts four file kinds — direct pattern, motif
  pattern, song file, and drum pattern — and auto-detects which by content,
  in the order song → drums → pattern/motif (`src/generate/mod.rs`).

## README.md — signpost the formats and fix the starter-patterns table

- [x] In the "Algorithmic generation" section (around `README.md:136`), add a
  one-or-two-sentence pointer that `generate --pattern` also accepts motif
  patterns, song files (`parts`/`arrangement`), and drum patterns
  (`voices`), with the full schema in `docs/algorithmic-generation.md`.

- [x] Update the "Starter patterns ship in `patterns/`" table
  (around `README.md:173`) so it covers the `motif/`, `song/`, and `drums/`
  families — either add rows for them or group by family — so the table no
  longer implies only the 6 direct-pattern files exist. Keep it accurate to
  the 23 files currently under `patterns/`.
