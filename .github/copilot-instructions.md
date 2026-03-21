# Copilot Repository Instructions

## Canonical References
Use these files as the primary guidance:
1. `AGENTS.md`
2. `NAVIGATOR.md`
3. Optional local `d2r-spec/AGENTS.md`
4. Optional local `d2r-spec/AI_WORKFLOW.md`
5. The active `d2r-spec/.agents/tasks/*.md` file, if it exists
6. Relevant `d2r-spec/discussion/*.md`

If instructions conflict, prefer the earlier file in the list.

## Repo Expectations
- `AGENTS.md` is the strategic policy source.
- `d2r-spec/AI_WORKFLOW.md`, if present locally, defines the private multi-model operating flow.
- `d2r-spec/AGENTS.md`, if present locally, is a private overlay and must not be required for public repo readability.
- `d2r-spec/` and fixtures are the truth source for binary behavior.
- `src/bin/verify/*` and `cargo test` are the preferred verification paths.

## Working Rules
- Keep changes atomic and verifier-backed.
- Do not widen scope beyond the active task spec.
- If `d2r-spec/AGENTS.md`, `d2r-spec/AI_WORKFLOW.md`, or `d2r-spec/.agents/tasks/` exists locally, use them only as an extra private overlay and do not copy private detail into public-facing docs by default.
- If the task spans `3+ files` or reopens bit-level ambiguity, stop for a planning step in `d2r-spec/.agents/tasks/` when available and recommend delegation to a stronger secondary model for the main implementation pass.
- Even with a task spec, only execute explicitly bounded executor-safe slices.
- Do not invent offsets, bit widths, or file-layout assumptions when fixture evidence is missing.
- Prefer relative-path references in markdown docs.

## Completion Format
When you finish a meaningful unit of work, include:
- `Verification:`
- `Recommended Next Model:`
