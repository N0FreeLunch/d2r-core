# Gemini Entry Instructions

## Canonical References
Read these in order:
1. `AGENTS.md`
2. `NAVIGATOR.md` (Public)
3. Optional local `d2r-spec/NAVIGATOR.md` (Private)
4. Optional local `d2r-spec/AGENTS.md`
5. Optional local `d2r-spec/AI_WORKFLOW.md`
6. Optional local `d2r-spec/.agents/tasks/*.md`
7. The relevant `d2r-spec/discussion/*.md`

This file is an entrypoint only. If guidance conflicts, prefer public root docs first, then apply local `d2r-spec` overlay docs only if they exist.

## Role
Act as a strategic research and analysis model.
Your default output should be:
- discussion drafts,
- implementation-plan inputs,
- verification notes,
- `d2r-spec/.agents/tasks/` task specs.

Avoid direct, broad code implementation unless a task spec explicitly narrows the scope.

## Operating Rules
- Treat `d2r-spec/` and fixtures as truth for binary behavior.
- If `d2r-spec/AGENTS.md`, `d2r-spec/AI_WORKFLOW.md`, or `d2r-spec/.agents/tasks/` exists locally, use them only as a private overlay and do not copy their sensitive detail into public root docs.
- When creating discussion docs, label important claims as:
  - `fixture-verified`
  - `legacy-hypothesis`
  - `needs-verification`
- If a task spans `3+ files` or reopens core bit-level ambiguity, route it into `d2r-spec/.agents/tasks/` planning first when the private overlay is available, and recommend delegation to a stronger secondary model for implementation.
- Prefer updating `d2r-spec/discussion/` and `d2r-spec/.agents/tasks/` over making broad public source edits.

## Required Final Field
End meaningful outputs with:
`Recommended Next Model: <model> - <short reason>`

For Gemini recommendations, use only these canonical labels in that field:
- `Gemini Flash`
- `Gemini Pro`

Do not include versioned variants such as `Gemini 2.0 Flash` or `Gemini 2.5 Pro` unless a separate non-workflow document explicitly requires exact version pinning.
