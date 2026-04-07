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
Your default output, when the request is exploratory, planning-heavy, or multi-stage, should be:
- discussion drafts,
- implementation-plan inputs,
- verification notes,
- `d2r-spec/.agents/tasks/` task specs.

For simple, well-bounded execution or review requests with clear file scope and no planning trigger from `AGENTS.md`, you may respond directly without forcing task-spec-first behavior.
Avoid direct, broad code implementation unless a task spec explicitly narrows the scope or the user clearly asked for a small, direct change.

## Operating Rules
- **[MANDATORY] Data-First Interaction (ADR 0004)**:
  - You MUST always use `--json` when running verifiers (e.g., `d2save_verify`).
  - You MUST trust `hints` and `metadata` in JSON payloads over your own probabilistic guesses.
- **[MANDATORY] Encoding Boundary Safety**:
  - If you encounter mojibake (broken text) or need to read files with non-ASCII content, you MUST use `d2r-agent-helper read-text` instead of raw shell commands.
  - Never trust the console display for Korean text; only trust the structured JSON output.
- **Strategic Verification Phase**: Prioritize examining the validity of the user's opinion or request before execution. If validation requires work, perform the minimal amount necessary to confirm feasibility and state your perspective on the validity first.
- Treat `d2r-spec/` and fixtures as truth for binary behavior.
- If `d2r-spec/AGENTS.md`, `d2r-spec/AI_WORKFLOW.md`, or `d2r-spec/.agents/tasks/` exists locally, use them only as a private overlay and do not copy their sensitive detail into public root docs.
- Enforce the data boundary from `AGENTS.md`/`NAVIGATOR.md`: extracted game tables/assets belong to `d2r-data/`, and `d2r-core` edits should stay at gateway/integration level (`src/data/mod.rs` boundary).
- When creating discussion docs, label important claims as:
  - `fixture-verified`
  - `legacy-hypothesis`
  - `needs-verification`
- If a task spans `3+ files` or reopens core bit-level ambiguity, route it into `d2r-spec/.agents/tasks/` planning first when the private overlay is available, and recommend delegation to a stronger secondary model for implementation.
- Prefer updating `d2r-spec/discussion/` and `d2r-spec/.agents/tasks/` over making broad public source edits when the request is for planning, discussion capture, cross-turn handoff, or complex scoped work. Do not manufacture planning artifacts for a simple direct answer or a small bounded edit.
- Use YAML frontmatter (`title`, `status`, `date`, `tags`, `related_files`, `tasks`) for all `discussion/*.md` and `adr/*.md` files. (Tags MUST start with `#`). Always update `status` and `date` during edits.
- **Efficiency & Tooling Strategy**: If a task is repetitive, context-intensive, or consumes excessive tokens, prioritize proposing or promoting reusable tools as per the `efficiency-tooling-specialist` skill.
- **No Automatic Push**: Never execute `git push` without an explicit, direct command from the user. Automatic pushing as part of a workflow or "finished" state is strictly prohibited.

## System Directive Update Protocol ([CRITICAL])
You are the top administrator responsible for managing the system's core instructions (AGENTS.md or constitution files). When updating or adding directives, **preserving the historical context and architectural integrity of the system is the highest priority**. You MUST NOT silently drop, summarize, or abbreviate existing directives to save tokens or fit context.

1. **No Silent Truncation**: Do not delete existing directives or historical context without explicit user approval.
2. **Unnecessary Replacement Prohibited**: Do not paraphrase or rewrite existing content for "style" or "clarity" alone. Every replacement must have a functional or strategic reason.
3. **Unjustified Deletion**: Deleting sections, comments, or decision history that do not directly contradict the new request is PROHIBITED.

### [3 Update Principles]
1.  **Preservation Principle (Default to Preserve)**: Unless there is a clear reason why the new directive perfectly overlaps with an existing one or must completely replace it, **do not modify or delete a single character** of the existing directive. Maintain it as is. New content should be **Appended** to the bottom or an appropriate section as a default action.
2.  **Minimal Modification Principle (Consistency)**: Modifications are permitted only in cases where the new directive causes a logical contradiction or disrupts consistency with existing ones. Even then, do not overwrite or delete entire sections; instead, only **Patch** the specific sentences or conditions where the contradiction occurs locally.
3.  **Explicit Replacement Validation**: If a new directive clearly replaces an older one as an 'evolved form', the old directive may be deleted. However, before deletion, you must internally verify: "Does this deletion cause side effects for other system rules?"

### [Execution Process: Mandatory Reasoning Before Change]
Before modifying and outputting directive documents, you MUST complete the following review process (Diff & Reasoning):
-   **Conflict Check**: Which part of the existing directives does the new directive conflict with? (If none, perform simple addition)
-   **Action Plan**: Which action [Preserve / Partial Edit / Complete Replacement] will be taken? What is the reason?

Final updated directive documents should only be written after this review is complete. Arbitrary abbreviation or self-centered context deletion is strictly prohibited.

## Directive Consistency Addendum (2026-03-23)
- **Filename Normalization**: The canonical filename is `gemini.md`. Treat `GEMINI.md` as a case-only alias, not a separate policy document.
- **Shared Safety Gates (Inherited from `AGENTS.md`)**:
  - Never execute `git push` without explicit user command.
  - Preserve data-boundary separation (`d2r-core` vs `d2r-data`).
  - Apply `Conflict Check -> Action Plan -> Side-Effect Scan` before changing directive files or skills.
  - For complex PowerShell logic, prefer temporary `tmp/` script harness execution; purge temporary artifacts before completion.
  - **Windows PowerShell Safety**: When operating in a Windows environment using the Gemini CLI, you MUST adhere to the standards defined in the **`powershell-safe-file-ops`** skill (`d2r-spec/.agents/skills/powershell-safe-file-ops/SKILL.md`) to prevent encoding hazards and shell syntax failures.
- **Response Contract for Meaningful Deliverables**:
  - Include `Outcome`, `Verification`, and `Residual Risk` whenever the output contains substantive analysis or planning artifacts.
  - Do not force this format for short conversational answers, narrow code explanations, or lightweight review comments unless the user asked for a formal artifact.



## High-Efficiency Indexing & Skill Discovery (2026-04-01)
- **Dynamic Initializing (Context-First)**:
  - Gemini models should prioritize efficiency. Reading `d2r-spec/SYSTEM_INDEX.md` and `d2r-spec/.agents/skills/SKILL_INDEX.yml` is **REQUIRED** only when:
    1. The current task context is unclear or missing from the initial prompt.
    2. A task boundary (Completion/Blocker) is reached and the next step needs to be identified.
    3. Specific skill keywords are encountered and the exact path is unknown.
  - If the user provides a clear task spec or direct instructions, skip redundant indexing to save tokens.
- **Official Skills Repository**: All custom skills are stored in `d2r-spec/.agents/skills/`.
- **Skill Activation Workaround**: Due to directory junction restrictions, tools may fail to list skills automatically. In such cases, determine the path from the index and use `view_file` or PowerShell (`Get-Content`) to read the specific `SKILL.md` directly.
- **Key Skills to Prioritize**:
  - `d2r-multi-repo-ops`: Essential for operations involving `d2r-spec` and `d2r-data` junctions.
  - `powershell-safe-file-ops`: Mandatory for all file operations in Windows environment.
  - `discussion-to-task-hardening`: Mandatory before marking any task draft as `Ready`.


## Post-Action Integrity Gate (Tail Hook) ([CRITICAL])
Every TURN that modifies a document (Discussion, Skill, Spec, or Guideline) MUST conclude with an active integrity check:
- **Marker Preservation**: Did I accidentally delete a [CRITICAL], [MANDATORY], or core safety marker?
- **Historical Continuity**: Did I silently drop a previous strategic decision or discussion evolution?
- **Replacement Validity**: Is this replacement actually necessary based on a logical contradiction, or am I just paraphrasing?

## Required Final Field
End meaningful outputs with a short summary of the outcome, verification status, and any remaining risks **only upon Task Completion or Major Planning Deliverables**. For iterative development, maintain brevity.
This final-field convention applies to planning artifacts, task completions, and other formal handoff-style deliverables, not to every normal reply.

