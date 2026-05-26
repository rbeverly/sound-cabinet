## Why

A `.sc` score that defines a section referring to itself — directly or
transitively — causes the expander to recurse infinitely until the
thread stack is exhausted, aborting the process with `SIGABRT` /
`thread 'main' has overflowed its stack`.

The bug lives on two parallel recursion paths in
`src/dsl/expand.rs`:

- **Expansion path.** `ExpansionContext::expand_section`
  (`src/dsl/expand.rs:262`) iterates each `SectionEntry` and calls
  `expand_pattern_ref` (`src/dsl/expand.rs:229`) for nested `play` /
  `repeat` / `sequence` entries. `expand_pattern_ref` dispatches to
  `expand_name` (`src/dsl/expand.rs:217`), which calls
  `expand_section` again when the name resolves to a section. There
  is no visited-set and no depth bound, so a cycle re-enters the same
  section unbounded.
- **Duration path.** Implicit-duration sections trigger
  `ExpansionContext::compute_section_duration`
  (`src/dsl/expand.rs:143`), which calls `duration_of_ref` →
  `duration_of` (`src/dsl/expand.rs:127`) → `compute_section_duration`
  for any nested `Play` / `AtPlay` / `Sequence` / `RepeatBlock` entry.
  The same lack of cycle detection means a self-referencing
  implicit-duration section stack-overflows here before expansion is
  even attempted.

The grammar permits both shapes — for example:

```
section loop
  play loop

play loop
```

or a transitive cycle (`section a` plays `b`; `section b` plays `a`).
The `play` top-level statement parses as a `pattern_ref` and the
expander resolves it to whichever name the user supplied; the parser
does not check whether that name refers to the enclosing section.

The harm is a denial-of-service crash on a user-controlled input:
anyone running `sound-cabinet play <file>.sc`,
`sound-cabinet render <file>.sc`, `sound-cabinet freeze <file>.sc`, or
`sound-cabinet export <file>.sc` against a crafted score gets a stack
overflow rather than a parse error. The fix is to track the set of
section names currently being expanded (or computed) and return an
`Err("Circular section/pattern reference: ...")` when a cycle is
re-entered — mirroring the existing circular-import check in
`src/dsl/import.rs`.

## What Changes

- Add cycle detection inside `ExpansionContext` (or as a parameter
  threaded through the recursive calls) covering both the expansion
  path (`expand_name` / `expand_section` / `expand_pattern_ref`) and
  the duration-computation path (`duration_of` /
  `compute_section_duration` / `duration_of_ref`).
- When a section or pattern name is encountered while it is already on
  the active expansion / duration stack, return an `anyhow::Error`
  naming the cycle (e.g. `"Circular reference: loop -> loop"` or
  `"Circular reference: a -> b -> a"`) instead of recursing further.
- Patterns that reference only their own events do not recurse and
  remain unaffected; the check applies only when a name lookup
  re-enters a section / pattern that is already being expanded.
- Add unit tests in `src/dsl/expand.rs` covering:
  the direct self-reference (`section loop` plays `loop`),
  the two-step cycle (`section a` plays `b`; `section b` plays `a`),
  and the implicit-duration variant so the duration-computation path
  is also exercised.

## Impact

- `src/dsl/expand.rs` — the recursion-detection state, the new error
  paths, and the tests.
- `openspec/specs/dsl-syntax/spec.md` — adds the invariant that the
  expander returns an error rather than panicking on a circular
  section/pattern reference.
- No grammar change. No change to scripts that today expand
  successfully.
