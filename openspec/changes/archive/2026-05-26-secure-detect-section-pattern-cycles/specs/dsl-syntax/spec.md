## ADDED Requirements

### Requirement: Section and pattern reference cycles rejected

The expander SHALL detect cycles in section and pattern references
during both implicit-duration computation AND event expansion. When
expanding a name `N` would re-enter another expansion of `N` that is
already on the active stack, the expander SHALL return a
`Result::Err` whose message identifies the cycle (e.g. `Circular
reference: a -> b -> a`). The expander SHALL NOT recurse further;
under no input MAY it terminate the process with a stack overflow.

The check applies to:

- Top-level `play <name>` statements where `<name>` is a section that
  transitively references itself.
- `SectionEntry::Play`, `SectionEntry::AtPlay`,
  `SectionEntry::AtRepeat`, `SectionEntry::Sequence`,
  `SectionEntry::RepeatEvery`, and `SectionEntry::RepeatBlock`
  whose `pattern_ref` resolves to an ancestor on the current
  expansion stack.
- Duration lookups for implicit-length sections, where
  `compute_section_duration` walks the same graph.

Independent top-level `play` statements that each reference the same
non-cyclic section SHALL continue to expand without error: the
cycle-tracking state is scoped to a single recursive descent, not
shared across sibling top-level statements.

#### Scenario: Direct self-reference rejected
- **GIVEN** a `.sc` file containing
  ```
  section loop
    play loop

  play loop
  ```
- **WHEN** the DSL expander processes the parsed script
- **THEN** `expand_script` returns `Err` whose message contains
  `Circular reference`
- **AND** the process does not crash with a stack overflow

#### Scenario: Two-step cycle rejected
- **GIVEN** a `.sc` file where `section a` plays `b` and
  `section b` plays `a`, and the script ends with `play a`
- **WHEN** the DSL expander processes the parsed script
- **THEN** `expand_script` returns `Err` whose message names both
  `a` and `b` in the cycle chain

#### Scenario: Implicit-duration cycle rejected
- **GIVEN** a section with no explicit `= N beats` duration whose
  body plays itself
- **WHEN** any caller asks for that section's duration (e.g. via
  `play sample(section, 0, 4)`) or expands it
- **THEN** the operation returns `Err` rather than recursing into
  `compute_section_duration` until the stack overflows

#### Scenario: Independent references to the same non-cyclic section permitted
- **GIVEN** a `.sc` file with one acyclic `section verse` and two
  top-level `play verse` statements
- **WHEN** the DSL expander processes the script
- **THEN** both statements expand successfully — the cycle-tracking
  state from the first does not block the second
