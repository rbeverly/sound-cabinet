## 1. Capture the baseline

- [x] 1.1 Survey the existing Rust implementation (src/, docs/, README) and identify the 11 user-facing capabilities.
- [x] 1.2 Author one delta spec per capability with `## ADDED Requirements` containing all requirements that describe the current behavior. Each requirement uses SHALL / SHALL NOT / MAY normatively. Each requirement has at least one `#### Scenario:` block.
- [x] 1.3 Cross-check requirements against the actual code (parser grammar, CLI argv parsing in `src/main.rs`, engine code, docs) so the spec is descriptive of current behavior, not aspirational.

## 2. Validation

- [x] 2.1 `openspec validate 2026-05-21-initial-spec-baseline --strict` passes.
- [x] 2.2 Each spec file has the `# <capability> Specification` title and `## ADDED Requirements` header, followed by one or more requirements.
- [x] 2.3 No requirement uses imperative-without-SHALL ("the parser MUST" / "the engine should") — every normative line uses SHALL, SHALL NOT, or MAY.

## 3. Acceptance

- [x] 3.1 This change is archived in place at `openspec/changes/archive/2026-05-21-initial-spec-baseline/`. There is no active-change-then-archive workflow because the implementation already exists; this entry is a retroactive capture so the archive is authoritative.
- [x] 3.2 The dated folder name precedes any 2026-05-22 archive entry so that archive replay in date order applies the baseline before subsequent modifications.
