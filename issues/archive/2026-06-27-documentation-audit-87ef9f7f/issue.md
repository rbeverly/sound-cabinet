# Document the motif, song, and drum-pattern generation formats

## Problem

`sound-cabinet generate --pattern <file>` auto-detects and renders four kinds
of YAML input (`src/generate/mod.rs::run_generate` dispatches by file content:
a `parts` key → song file, a `voices` key → drum pattern, otherwise a
pattern/motif file). Three of these four formats ship with working examples
but are completely undocumented:

- **Motif patterns** — a `motif` block (`rhythm`/`contour`/`emphasis`) plus
  either a `structure` list of named transformations or a `complexity` level.
  Expanded by `src/generate/motif.rs`. Examples: `patterns/motif/` (4 files).
- **Song files** — a top-level `parts` map (each part carries a `motif`,
  optional `structure`/`complexity`, a per-part `chords` override, and a
  per-part `range`) plus an `arrangement` list naming part order. Rendered by
  `src/generate/song.rs`. Examples: `patterns/song/` (3 files).
- **Drum patterns** — a `voices` list, each voice carrying `voice`, `pitch`,
  `rhythm`, and an optional `emphasis`. Rendered by
  `src/generate/drums.rs`. Examples: `patterns/drums/` (4 files).

Neither `README.md`'s "Algorithmic generation" section nor
`docs/algorithmic-generation.md` (the page the README points to for "how to
write your own patterns") mentions any of these keys, the transformation
vocabulary, the complexity levels, or that `generate --pattern` accepts song
and drum files at all. The README's "Starter patterns ship in `patterns/`"
table (`README.md:173`) lists only 6 of the 23 shipped pattern files and omits
the entire `patterns/drums/`, `patterns/song/`, and `patterns/motif/`
families, so operators have no signpost to these generation modes.

All three formats are already specified in the canonical `song-generation`
spec (the `Pattern files must be internally consistent`, `Song files must
define parts and a valid arrangement`, `Drum patterns must define voices with
consistent emphasis`, and `Motif expansion must reject missing motifs and
unknown transformations` requirements). The behavior is correct and shipping;
only the operator-facing documentation is missing.

## Desired end state

An operator can author a motif pattern, a song file, and a drum pattern from
the docs alone:

- `docs/algorithmic-generation.md` documents the motif format (the `motif`
  block, the `structure` transformation vocabulary, and the `complexity`
  levels with their auto-expanded structures), the song-file format
  (`parts`/`arrangement`, per-part `chords` and `range` overrides), and the
  drum-pattern format (`voices` with `voice`/`pitch`/`rhythm`/`emphasis`),
  and notes that `generate --pattern` auto-detects which kind a file is.
- `README.md`'s "Algorithmic generation" section signposts that song and drum
  files are also accepted, and its starter-patterns table covers (or points
  to) the `motif/`, `song/`, and `drums/` families rather than only the 6
  direct-pattern files.

This issue carries no spec delta — the formats are already canon; only the
docs change.
