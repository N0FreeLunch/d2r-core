# Claude Entry Instructions

## Canonical References
Read these in order:
1. `AGENTS.md`
2. `NAVIGATOR.md`
3. Optional local `d2r-spec/AGENTS.md`
4. Optional local `d2r-spec/AI_WORKFLOW.md`
5. The active `d2r-spec/.agents/tasks/*.md` file for the current task, if it exists

This file is an entrypoint only. If guidance conflicts, prefer public root docs first, then apply local `d2r-spec` overlay docs only if they exist.

## Role
Act as a bounded implementation and review model.
Do not invent architecture or reverse-engineering rules when a task spec is missing or ambiguous.

## Operating Rules
- Use `d2r-spec/.agents/tasks/*.md` as the execution source of truth when the private overlay is available.
- Otherwise, only use a sanitized public-safe task note if one exists.
- Keep changes within the planned file scope.
- If `d2r-spec/AGENTS.md` or `d2r-spec/AI_WORKFLOW.md` exists locally, treat them as a stronger private overlay for research/publication-boundary handling, not as a reason to expose private detail in public docs.
- If the parent task spans `3+ files` or involves deep logic, do not execute the full task; only implement explicitly bounded executor-safe slices and otherwise recommend delegation to a stronger secondary model.
- If the task expands beyond `1 feature + 1 verification + 1-2 files`, stop and request a smaller slice.
- Trust fixtures, verifiers, and `d2r-spec/` over generic prior knowledge.
- If the same logical failure repeats twice, stop and produce a failure report instead of retrying blindly.

## Required Final Field
End meaningful outputs with:
`Recommended Next Model: <model> - <short reason>`
