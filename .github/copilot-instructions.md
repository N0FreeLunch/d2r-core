# Copilot Repository Instructions

This file is a small public bootstrap for GitHub Copilot working in `d2r-core`.
It must stay concise and publishable. Detailed private workflow, indexing, skill discovery, and task-routing rules belong in the Strategy Hub resolved from `D2R_SPEC_PATH`.

## Canonical References
Use these files as the primary guidance:
1. `AGENTS.md` (public safety floor)
2. `NAVIGATOR.md` (public implementation and verification index)
3. If a local Strategy Hub is available, use it as the private operating context:
   - `<D2R_SPEC_PATH>/AGENTS.md`
   - `<D2R_SPEC_PATH>/AI_WORKFLOW.md`
   - relevant `<D2R_SPEC_PATH>/.agents/tasks/*.md`
   - relevant `<D2R_SPEC_PATH>/discussion/*.md`

If the Strategy Hub is absent or inaccessible, continue with only the public root documents and source tree. Do not block normal core usage on private overlay material.

## Strategy Hub Shape
When available, the Strategy Hub is expected to provide the private context that this public bootstrap intentionally does not duplicate:
- `AGENTS.md` and `AI_WORKFLOW.md` for workflow and publication-boundary rules;
- `NAVIGATOR.md` for private domain routing;
- `.agents/tasks/` for parent tasks and mini-spec handoffs;
- `discussion/`, `research/`, or local navigation/cache files for deeper provenance and recent-delta context.

Core-side Copilot output should use those files when present, but should not copy private detail back into public core documents.

## Core Work Must-Do Floor
Use this section when Copilot is assisting inside `d2r-core` from a wider `d2r` workspace where Strategy Hub instructions may not be automatically loaded.
- Resolve the Strategy Hub through `D2R_SPEC_PATH` first; if unset, check the sibling `../d2r-spec/` path.
- Before non-trivial core edits, consult the relevant Strategy Hub task or discussion context when available.
- For quick recent-delta context, prefer `<D2R_SPEC_PATH>/.agents/navigation/active-context.md` or `d2map-query context` when those tools/files are available; do not duplicate detailed indexing rules here.
- Use verifier JSON output (`--json`) when supported, and prefer structured diagnostics over guesses.
- If scope expands beyond a bounded implementation slice, stop for Strategy Hub planning instead of broad direct implementation.
- If the Strategy Hub is unavailable and the task depends on private fixture/research truth, report the missing context rather than inventing assumptions.

## Working Rules
- Keep changes atomic and verifier-backed.
- Preserve repository boundaries: `d2r-core` contains public implementation logic; extracted game data belongs in `d2r-data`; private reasoning belongs in `d2r-spec`.
- Do not invent offsets, bit widths, or file-layout assumptions when fixture evidence is missing.
- Prefer public verification tools documented in `NAVIGATOR.md`, plus targeted tests when code changes require them.
- If Strategy Hub guidance conflicts with public safety constraints, keep the public safety constraints intact.
- Never suggest or perform `git push` unless the user explicitly asks for it.

## Completion Format
For meaningful implementation or review results, include:
- `Verification:`
- `Residual Risk:`
