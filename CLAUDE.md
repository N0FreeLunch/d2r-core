# Claude Entry Instructions (Bootstrap)

## Canonical References
Read these in order:
1. `AGENTS.md` (Public Safety Floor)
2. `NAVIGATOR.md` (Public Index)
3. Optional local Strategy Hub files resolved from `D2R_SPEC_PATH` when that environment is available:
   - a spec-side `CLAUDE.md` companion note
   - the local `AGENTS.md` and `AI_WORKFLOW.md`
   - local `.agents/tasks/*.md` files when they are actually relevant

This document is a **bootstrap entrypoint**. Prefer public root docs first, then use any local Strategy Hub material resolved from `D2R_SPEC_PATH` only as an optional companion overlay when that environment exists.

## Role
Act as a bounded implementation and review model.
Do not invent architecture or reverse-engineering rules when a task spec is missing or ambiguous.

## Operating Rules
- When a local Strategy Hub is available, local task artifacts under `D2R_SPEC_PATH` may be used as execution context for bounded slices.
- Otherwise, only use a sanitized public-safe task note if one exists.
- Keep changes within the planned file scope.
- Enforce the data boundary from `AGENTS.md`/`NAVIGATOR.md`: extracted game tables belong to `d2r-data/`, while `d2r-core` should only use `src/data/mod.rs` as the gateway.
- If local `AGENTS.md` or `AI_WORKFLOW.md` files resolved from `D2R_SPEC_PATH` exist, treat them as an optional private overlay for research and publication-boundary nuance, not as a reason to expose private detail in public docs.
- If the parent task spans `3+ files` or involves deep logic, do not execute the full task; only implement explicitly bounded executor-safe slices and otherwise recommend delegation to a stronger secondary model.
- If the task expands beyond `1 feature + 1 verification + 1-2 files`, stop and request a smaller slice.
- Trust fixtures, verifiers, and any available local Strategy Hub notes over generic prior knowledge.
- If the same logical failure repeats twice, stop and produce a failure report instead of retrying blindly.

## Directive Consistency Addendum (2026-03-23)
- **Filename Normalization**: The canonical filename is `CLAUDE.md`. `cloude.md` is treated as a typo alias and MUST resolve to this file.
- **Shared Safety Gates (Inherited from `AGENTS.md`)**:
  - Never execute `git push` without explicit user command.
  - Keep strict repository/data boundaries (`d2r-core` implementation vs `d2r-data` tables/assets).
  - Before modifying directive files or skills, run `Conflict Check -> Action Plan -> Side-Effect Scan`.
  - For complex PowerShell operations, use temporary script-first harness flow in `tmp/`, then purge temporary artifacts.
- **Execution Output Contract**:
  - For implementation/review results, include `Outcome`, `Verification`, and `Residual Risk` before the required next-model line.

## Required Final Field
End meaningful outputs with:
`Recommended Next Model: <model> - <short reason>`
