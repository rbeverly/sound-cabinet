# Tasks: fix cyclic section references overflowing the stack

## 1. Detect cycles during section expansion
- [ ] 1.1 In `src/dsl/expand.rs`, thread a set of section names that are
  currently being expanded (an "active" / on-stack set) through
  `ExpansionContext::expand_name` and `expand_section`. A `HashSet<String>`
  (or a `&mut` borrow of one) carried into the recursion is sufficient.
- [ ] 1.2 In `expand_section` (`src/dsl/expand.rs:108`), before recursing into a
  section entry, check whether the target section name is already in the active
  set. If it is, return `Err(anyhow!("Circular section reference detected: '{name}'"))`
  rather than recursing.
- [ ] 1.3 Insert the section name into the active set on entry to
  `expand_section` and remove it on exit (or rely on the borrow scope) so that a
  section may still be played multiple times in non-nested positions, while a
  re-entrant (on-stack) reference is rejected. Patterns do not recurse, so only
  section names need tracking.
- [ ] 1.4 Apply the same guard to BOTH `SectionEntry::Play`
  (`src/dsl/expand.rs:141-149`) and `SectionEntry::RepeatEvery`
  (`src/dsl/expand.rs:120-140`).

## 2. Tests
- [ ] 2.1 Add a unit test `expand_rejects_self_referential_section` in
  `src/dsl/expand.rs` that builds a script with a `SectionDef` whose only entry
  is `SectionEntry::Play { name: "a", .. }` named `"a"`, followed by
  `PlaySequential { name: "a" }`, and asserts `expand_script` returns `Err`
  (and does not overflow the stack).
- [ ] 2.2 Add a unit test `expand_rejects_mutually_recursive_sections` covering
  section `a` playing `b` and section `b` playing `a`, asserting `Err`.
- [ ] 2.3 Add a unit test asserting that a section played twice from two
  independent (non-nested) top-level `play` statements still succeeds, to
  confirm the active-set is scoped to the call stack and not a global "seen"
  set.
- [ ] 2.4 Confirm existing tests `test_expand_section`,
  `expand_rejects_zero_repeat_interval`, and
  `expand_rejects_negative_repeat_interval` still pass.
