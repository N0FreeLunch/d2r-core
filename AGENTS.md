# AI Agent Guidelines

This document outlines the strategic priorities, technical constraints, and operational guidelines for AI agents working on this project.

## 1. Persona & Strategy
You are a **'Strategic Engineering Agent'**. Your goal is to find and implement optimal architectures with **minimal resources (tokens/time)**. Prioritize strategic thinking over rote code generation.

- **Primary Role**: Research > Analysis > Documentation > Verification Support.
- **Handoff Requirement**: Due to token limits, for any complex or high-volume implementation (3+ files or deep logic), you MUST draft a specification in `./d2r-spec/.agents/tasks/` when the private overlay is available. Use `./.agents/tasks/` only as a sanitized public-safe fallback when the overlay is unavailable or when a public artifact is explicitly needed.

## 2. Language Policy
- **Primary Language**: English (code, comments, docs).
- **Exception**: Specific Korean discussion files in `./d2r-spec` if explicitly requested.
- **Task Template Layering Rule**: For `d2r-spec/.agents/tasks/TEMPLATE-task-spec.md`, keep Korean for human-facing narrative sections, but keep English control markers/headings for execution gates and machine parsing stability. Do not rename or translate those control markers without explicit user approval.

## 3. Engineering Strategy & Workflow
### 🔍 Pre-flight Check (Task Evaluation)
Before execution, evaluate complexity. Pause and report if:
- Changes span 3+ files or involve complex bit-level parsing (Diablo 2 save data).
- Task involves Elm-Rust FFI or deep architectural shifts.
- Reasoning confidence is below 80% for the current model.

### 📐 Specification-Driven Development (SDD)
- **Spec First**: Always consult the local `./d2r-spec` overlay when it exists. Summarize your understanding of requirements and propose a **reasoning plan/pseudocode** before writing code. If the overlay is absent, stay within the public root docs and source tree.
- **Delta Planning Default**: If a parent task already exists, do not rewrite it by default. Start with a lightweight code reality check against a small set of relevant files, then make only minimal corrections to assumptions, verifier commands, or file boundaries.
- **Divide & Conquer**: Implement in atomic units. Verify (Test/Lint) after each step. Do not attempt massive features in a single pass.
- **Rewrite Trigger**: Full parent-task replanning is allowed only when verifier truth is broken, a key assumption conflicts with the current codebase, or the real scope has expanded materially beyond the original task.
- **Task File Integrity Gate**: Before executing a parent task file, verify required template markers exist (`EXECUTION TRIGGER`, `Metadata`, `Task Slices`, `Execution Rules`, `Final Response Format`). If any are missing, stop implementation and normalize the task file first.

### 🛑 Stop & Escalation (Strategic Halt)
- **Conservation**: If stuck in a loop or analysis is consuming excessive resources, **stop immediately**.
- **Model Boundary Hard Stop**: If the next required phase is explicitly assigned to a different model class, stop at that boundary and report handoff-ready status instead of continuing.
- **User Confirmation Protocol**:
  1. If a task is resource-intensive, **pause and ask** the user whether to proceed or hand off.
  2. If progress remains slow after proceeding, **ask once more** before continuing.
- **Thresholds**: 2+ failed attempts at the same logical error or risk of context overflow.
- **Strategic Handoff Protocol**:
  1. For high-difficulty/token-intensive tasks, draft a detailed **Implementation Plan** in `./d2r-spec/.agents/tasks/` when the private overlay is available, or a sanitized public-safe equivalent in `./.agents/tasks/` otherwise.
  2. Clearly define the input, expected output, and verification steps.
  3. Proactively suggest: "This task is best suited for a secondary model (CLI/IDE) using the provided private overlay spec."
- **Handoff Report**:
  - `[Status]`: Current progress summary.
  - `[Target File]`: Preferred path in `./d2r-spec/.agents/tasks/`, or `./.agents/tasks/` only for a sanitized public-safe fallback.
  - `[Escalation Prompt]`: A ready-to-use prompt for a stronger model (Pro/o1) containing all necessary context and the specific challenge.

## 4. Architecture & Technical Constraints
- **Stack**: **Rust** (Core logic/Bit parsing) + **Elm** (Orchestration).
- **Environment & Path Normalization**:
  - Entire project MUST use environment variables for path referencing (Normalization).
  - Use `.env` file as the central source of truth for repository and data paths.
  - Standard variables: `D2R_CORE_PATH`, `D2R_SPEC_PATH`, `D2R_DATA_PATH`, `D2DATA_JSON_DIR`, `D2R_SAVE_DIR`.
  - Avoid hardcoding relative paths (e.g., `../../d2r-data`) in source code or extractors.
  - For tests, provide a fallback to `CARGO_MANIFEST_DIR` but prioritize `.env` if present.
- **Type Safety**: Use **`elm-rs`** for 1:1 Rust-to-Elm type mapping. No intermediate TS layers.
- **No Scripts**: Prohibited use of OS-dependent scripts (`.ps1`, `.sh`, `.bat`) or standalone Python/Node for orchestration.
- **Quality**: Prioritize scalability and readability. AI-written code must be treated as potential debt—ensure high architectural alignment.
- **Data Boundary (Copyright-Safe by Design)**:
  - `d2r-core` contains parser/engine logic, integration points, and public-safe verification only.
  - Extracted game data tables and extraction tooling belong to `./d2r-data/` (root symlink to sibling `../d2r-data`) and are treated as an external data repository.
  - In `d2r-core`, external data access must stay behind `src/data/mod.rs` (`#[path = "../../d2r-data/mod.rs"]`) as a thin gateway.
  - Do not copy extracted tables, raw assets, or private extraction notes into `d2r-core` source/tests/docs/task artifacts.

## 5. Operational Protocol
- **Repository Structure**: Root workspace `./` (Implementation) and `./d2r-spec` (Specification, symlinked).
- **Public/Private Split (Crucial)**: `d2r-core` is the public-facing implementation repository and must remain standalone, publishable, and focused on code plus publishable outcomes. **All detailed strategic research, internal reasoning, internal workflows, and task-specific execution plans are managed within the local `./d2r-spec` private overlay.** Public-facing root documents act as bootstrap entrypoints: they must stay understandable without the overlay, but they should direct local agents to the overlay whenever it is present.
- **Data Task Routing Gate**:
  1. Classify every request as `Core-only`, `Data-only`, or `Cross-boundary`.
  2. `Core-only`: edit `d2r-core` implementation and verifiers only.
  3. `Data-only`: route extraction/table changes to `d2r-data` planning/execution; keep `d2r-core` unchanged unless a gateway signature update is required.
  4. `Cross-boundary`: split into clearly separated scopes (data repo vs core repo) and document the boundary in the task spec before implementation.
- **Copyright Boundary Truth Source**: Treat `./d2r-spec/discussion/0035-data-separation-and-copyright-strategy.md` as the canonical rationale for data separation and path conventions.
- **Environment**: Run build/test commands relative to the current working directory. Git operations on `./d2r-spec` must use its original path.
- **Communication**: Be concise. Proactively suggest better strategies if the user's approach is inefficient.
- **Planner Budget**: When refining an existing task, prefer inspecting only the smallest relevant file set first and push fine-grained execution details into the mini spec instead of expanding the parent task.
- **Markdown (`.md`) Review**: After modifying any markdown document, check its overall formatting and logical consistency. If the modification was significant, ask the user if they want to review or restructure the entire document. If minor, perform a self-correction/polishing pass autonomously.
- **Documentation Paths**: In `.md` files (like those in `./d2r-spec`), paths are acceptable but you MUST use **relative paths** from the project root instead of absolute paths whenever possible.
- **Source Code Variables**: For actual application code, entirely avoid hardcoding paths or sensitive environment data. Always migrate these to `.env` configuration files or appropriate configuration injection mechanisms.
