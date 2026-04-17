# Claude Entry Instructions (Bootstrap)

## Canonical References
Read these in order:
1. `AGENTS.md` (Public Safety Floor)
2. `NAVIGATOR.md` (Public Index)
3. **The `CLAUDE.md` file resolved from `D2R_SPEC_PATH`** (Local Strategy Hub - **Primary Authority when Hub is present**)
4. The `AGENTS.md` file resolved from `D2R_SPEC_PATH` (Workflow Overlay)
5. The `AI_WORKFLOW.md` file resolved from `D2R_SPEC_PATH`
6. The active `.agents/tasks/*.md` file under the path resolved from `D2R_SPEC_PATH`, if it exists

This document is a **bootstrap entrypoint**. If a local Strategy Hub exists at the path resolved from `D2R_SPEC_PATH`, its instructions take precedence for implementation, review, and verification workflows. If guidance conflicts, prefer public root docs (`AGENTS.md`) first, then apply the local Strategy Hub overlay only if it exists.

## Role
Act as a bounded implementation and review model.
Do not invent architecture or reverse-engineering rules when a task spec is missing or ambiguous.

## Operating Rules
- Use the `.agents/tasks/*.md` path resolved from `D2R_SPEC_PATH` as the execution source of truth when the private overlay is available.
- Otherwise, only use a sanitized public-safe task note if one exists.
- Keep changes within the planned file scope.
- Enforce the data boundary from `AGENTS.md`/`NAVIGATOR.md`: extracted game tables belong to `d2r-data/`, while `d2r-core` should only use `src/data/mod.rs` as the gateway.
- If the local `AGENTS.md` or `AI_WORKFLOW.md` files resolved from `D2R_SPEC_PATH` exist, treat them as a stronger private overlay for research/publication-boundary handling, not as a reason to expose private detail in public docs.
- If the parent task spans `3+ files` or involves deep logic, do not execute the full task; only implement explicitly bounded executor-safe slices and otherwise recommend delegation to a stronger secondary model.
- If the task expands beyond `1 feature + 1 verification + 1-2 files`, stop and request a smaller slice.
- Trust fixtures, verifiers, and the Strategy Hub resolved from `D2R_SPEC_PATH` over generic prior knowledge.
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
