## 1. Add cycle detection to expansion and duration computation

- [x] 1.1 In `src/dsl/expand.rs`, give `ExpansionContext` (or thread
  through the recursive calls) a `RefCell<Vec<String>>` /
  `RefCell<HashSet<String>>` representing the names currently on the
  active expansion stack. The container MUST be cleared on each
  top-level entry to `expand_script` so independent top-level
  `play <name>` statements do not collide.
- [x] 1.2 In `ExpansionContext::expand_name`
  (`src/dsl/expand.rs:217`), before dispatching to
  `expand_pattern_events` or `expand_section`, push the resolved name
  onto the active stack. If the name is already present, return
  `Err(anyhow!("Circular reference: {chain} -> {name}"))` where
  `chain` is the joined stack (e.g. `a -> b -> a`). Pop the name on
  every return path (success or error).
- [x] 1.3 In `ExpansionContext::duration_of` (`src/dsl/expand.rs:127`)
  and `compute_section_duration` (`src/dsl/expand.rs:143`), apply the
  same push / check / pop pattern using a separate or the same
  cycle-tracking container. The duration path MUST also report the
  same circular-reference error rather than recurse, since it runs
  before expansion when a section has an implicit duration.
- [x] 1.4 In `expand_pattern_ref` (`src/dsl/expand.rs:229`) and
  `duration_of_ref` (`src/dsl/expand.rs:251`), propagate errors from
  the cycle check; do not swallow them with `unwrap_or`.

## 2. Tests

- [x] 2.1 Add a unit test `expand_rejects_direct_section_self_cycle`
  in `src/dsl/expand.rs` that parses
  ```
  section loop
    play loop

  play loop
  ```
  and asserts `expand_script(...)` returns `Err` whose message
  contains `"Circular reference"`.
- [x] 2.2 Add a unit test
  `expand_rejects_two_step_section_cycle` that parses
  ```
  section a
    play b

  section b
    play a

  play a
  ```
  and asserts `expand_script(...)` returns `Err` whose message
  contains `"Circular reference"` and includes the names `a` and
  `b`.
- [x] 2.3 Add a unit test
  `compute_section_duration_rejects_self_cycle` covering the
  implicit-duration variant — a `section` with no `= N beats`
  clause whose body contains a `play` of itself. Assert the function
  returns `Err` (i.e. expansion does not stack-overflow before
  expand_section is even reached).

## 3. Spec update

- [x] 3.1 Add an `## ADDED Requirements` block to
  `openspec/specs/dsl-syntax/spec.md` introducing
  `Requirement: Section/pattern reference cycles rejected`, with a
  scenario asserting that a circular reference returns a parse-time
  error rather than panicking the process.
