# Fix infinite loop + unbounded memory when a section repeats every 0 beats

## Why

`src/dsl/expand.rs:127-132` tiles a pattern across a section with:

```rust
let mut beat = 0.0;
while beat < section.duration_beats {
    let cmds = self.expand_name(name, base_beat + beat, ...)?;
    output.extend(cmds);
    beat += every_beats;
}
```

`every_beats` comes straight from a `.sc` section entry
(`repeat <name> every <number> beats`). The grammar
(`src/dsl/grammar.pest:62`, `number = "-"? ~ ASCII_DIGIT+ ...`) accepts
`0` and negative values, and `parse_section_def`
(`src/dsl/parser.rs:469`) parses it with no lower bound.

When `every_beats <= 0.0` the loop counter never advances past
`section.duration_beats`, so the `while` loop spins forever. Each
iteration calls `expand_name`, which clones the pattern's events and
appends them to `output`, so memory grows without bound until the
process is OOM-killed or the host is exhausted.

A score as small as the following hangs `sound-cabinet` and balloons
memory until the machine is starved:

```
pattern p = 1 beats
  at 0 play sine(440) for 0.5 beats
section s = 16 beats
  repeat p every 0 beats
play s
```

**Harm:** denial of service (unbounded CPU + memory) when rendering,
playing, watching, or exporting an attacker-supplied or mistyped
`.sc` file. `watch` mode re-runs expansion on every file save, so a
malicious file dropped into a watched directory hangs the running
process. This path is reached by `render`, `play`, `watch`, and
`export` (all call `expand_script`).

## What Changes

- In `src/dsl/expand.rs`, `ExpansionContext::expand_section`, reject a
  non-positive `every_beats` before entering the tiling loop and return
  a descriptive `Err` instead of looping forever. This guarantees the
  loop always makes forward progress.

## Impact

- Affected code: `src/dsl/expand.rs` (`expand_section`).
- Behavior change: a `repeat … every <n> beats` entry with `n <= 0`
  now fails fast with a clear error rather than hanging. Valid scores
  (positive intervals) are unaffected.
- No public API or data-format change.
