# Cyclic section references overflow the stack during expansion

## Summary

The score expander recurses through section `play` entries with no cycle
detection or depth limit. A section whose entry plays itself (directly or
through a cycle of sections) drives unbounded recursion and crashes the process
with a stack overflow (SIGABRT/SIGSEGV) before any audio is produced.

## Source location

- `src/dsl/expand.rs:147` — `SectionEntry::Play { name, .. }` calls
  `self.expand_name(name, ...)` with no guard against re-entering a section that
  is already on the call stack.
- `src/dsl/expand.rs:101-102` — `expand_name` resolves a name against **both**
  patterns and sections, so a section entry that names a section recurses into
  `expand_section`.
- `src/dsl/expand.rs:136` — `SectionEntry::RepeatEvery` has the same defect: it
  also calls `expand_name(name, ...)` and can re-enter the same section.
- `src/dsl/parser.rs:482-484` — a section `play <ident>` entry stores the raw
  identifier with no restriction that it reference a *pattern*; any section name
  is accepted.

The existing `every_beats <= 0.0` guard at `src/dsl/expand.rs:129-133` prevents
the *zero-interval* hang but does nothing for cyclic references, which never
advance through that loop in the first place.

## Reproduction (attacker-controlled `.sc`)

```
section a = 16 beats
  play a
play a
```

Expansion registers section `a` (`expand.rs:264-265`), then the top-level
`play a` (`PlaySequential`) calls `expand_name("a")` → `expand_section` →
entry `play a` → `expand_name("a")` → `expand_section` → … with no depth bound,
overflowing the stack. A mutual cycle (`section a` plays `b`, `section b` plays
`a`) triggers the identical crash.

The score file is untrusted input: it reaches `expand_script` via every command
that builds an engine or exports — `build_engine` / `load_definitions`
(`src/main.rs:315`, `:341`) and `run_export` (`src/export/mod.rs:44`).

## Harm

Denial of service: a single crafted score file crashes `sound-cabinet`
(`render`, `play`, `watch`, `piano`, `profile`, `export`) via stack overflow.

## Acceptance criteria

Stated against the existing canonical requirement **"Section repeat intervals
must be positive"** in `openspec/specs/score-expansion/spec.md`, which already
mandates the invariant *"Expansion of a section SHALL always terminate."* This
fix makes the code conform to that invariant for cyclic references; it changes
no observable contract for valid (acyclic) scores.

1. Expanding a section whose `play`/`repeat` entry forms a cycle (including a
   section that references itself) returns an `Err` identifying the offending
   section name, rather than recursing without bound.
2. The process does not crash with a stack overflow on any score file.
3. Valid, acyclic scores expand exactly as before — no change to the emitted
   `PlayAt`/`SetBpm` command stream for any existing passing test, and the
   existing `expand_rejects_zero_repeat_interval` /
   `expand_rejects_negative_repeat_interval` tests still pass.
