# Claude Entry Instructions (Public Bootstrap)

This file is a small public bootstrap for Claude-class agents working in `d2r-core`.
It must stay concise and publishable. Detailed private workflow, indexing, skill discovery, and task-routing rules belong in the Strategy Hub resolved from `D2R_SPEC_PATH`.

## Canonical References
Read these in order:
1. `AGENTS.md` (public safety floor)
2. `NAVIGATOR.md` (public implementation and verification index)
3. If a local Strategy Hub is available, prefer its model-specific authority:
   - `<D2R_SPEC_PATH>/CLAUDE.md`
   - `<D2R_SPEC_PATH>/AGENTS.md`
   - `<D2R_SPEC_PATH>/AI_WORKFLOW.md`
   - relevant `<D2R_SPEC_PATH>/.agents/tasks/*.md`

If the Strategy Hub is absent or inaccessible, continue with only the public root documents and source tree. Do not block normal core usage on private overlay material.

## Strategy Hub Shape
When available, the Strategy Hub is expected to provide the private operating context that this public bootstrap intentionally does not duplicate:
- model entrypoints such as `GEMINI.md` or `CLAUDE.md`;
- `NAVIGATOR.md` for private domain routing;
- `AGENTS.md` and `AI_WORKFLOW.md` for workflow and publication-boundary rules;
- `.agents/tasks/` for parent tasks and mini-spec handoffs;
- `discussion/`, `research/`, or local navigation/cache files for deeper provenance and recent-delta context.

Core-side agents should use those files when present, but should not copy private detail back into public core documents.

## Role
Act as a bounded implementation and review model.
Use this core-side file only to enter the public project safely; use the spec-side `CLAUDE.md` for detailed Claude workflow when available.

Keep implementation within the planned scope. If the work expands beyond a bounded slice, stop and request a smaller slice or planning pass.

## Public Safety Rules
- Never execute `git push` without an explicit user command.
- Preserve repository boundaries: `d2r-core` contains public implementation logic; extracted game data belongs in `d2r-data`; private reasoning belongs in `d2r-spec`.
- Keep public docs free of private reverse-engineering detail. Link or summarize only public-safe conclusions.
- Prefer verifier-backed conclusions over guesses. Use public verification tools documented in `NAVIGATOR.md`.
- If Strategy Hub guidance conflicts with public safety constraints, keep the public safety constraints intact.

## Directive Updates
Before modifying directive files such as `AGENTS.md`, `GEMINI.md`, `CLAUDE.md`, or `CONSTITUTION.md`, follow the public `AGENTS.md` directive-update protocol: conflict check, action plan, side-effect scan, then the smallest safe edit.
