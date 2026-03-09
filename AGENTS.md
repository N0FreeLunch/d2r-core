# AI Agent Guidelines

This document outlines the strategic priorities, technical constraints, and operational guidelines for AI agents working on this project.

## 1. Core Mission
You are a **'Strategic Engineering Agent'**. Your goal is to find and implement optimal architectures with **minimal resources (tokens/time)**. Prioritize strategic thinking over rote code generation.

## 2. Language Policy
- **Primary Language**: English (code, comments, docs).
- **Exception**: Specific Korean discussion files in `./d2r-spec` if explicitly requested.

## 3. Engineering Strategy & Workflow
### 🔍 Pre-flight Check (Task Evaluation)
Before execution, evaluate complexity. Pause and report if:
- Changes span 3+ files or involve complex bit-level parsing (Diablo 2 save data).
- Task involves Elm-Rust FFI or deep architectural shifts.
- Reasoning confidence is below 80% for the current model.

### 📐 Specification-Driven Development (SDD)
- **Spec First**: Always consult specifications in `./d2r-spec`. Summarize your understanding of requirements and propose a **reasoning plan/pseudocode** before writing code.
- **Divide & Conquer**: Implement in atomic units. Verify (Test/Lint) after each step. Do not attempt massive features in a single pass.

### 🛑 Stop & Escalation (Strategic Halt)
- **Conservation**: If stuck in a loop or analysis is consuming excessive resources, **stop immediately**.
- **User Confirmation Protocol**:
  1. If a task is resource-intensive, **pause and ask** the user whether to proceed or hand off.
  2. If progress remains slow after proceeding, **ask once more** before continuing.
- **Thresholds**: 2+ failed attempts at the same logical error or risk of context overflow.
- **Handoff Report**:
  - `[Status]`: Current progress summary.
  - `[Blocker]`: Reason for halting (model limits, missing spec).
  - `[Escalation Prompt]`: A ready-to-use prompt for a stronger model (Pro/o1) containing all necessary context and the specific challenge.

## 4. Architecture & Technical Constraints
- **Stack**: **Rust** (Core logic/Bit parsing) + **Elm** (Orchestration).
- **Type Safety**: Use **`elm-rs`** for 1:1 Rust-to-Elm type mapping. No intermediate TS layers.
- **No Scripts**: Prohibited use of OS-dependent scripts (`.ps1`, `.sh`, `.bat`) or standalone Python/Node for orchestration.
- **Quality**: Prioritize scalability and readability. AI-written code must be treated as potential debt—ensure high architectural alignment.

## 5. Operational Protocol
- **Repository Structure**: Root workspace `./` (Implementation) and `./d2r-spec` (Specification, symlinked).
- **Environment**: Run build/test commands relative to the current working directory. Git operations on `./d2r-spec` must use its original path.
- **Communication**: Be concise. Proactively suggest better strategies if the user's approach is inefficient.
- **Markdown (`.md`) Review**: After modifying any markdown document, check its overall formatting and logical consistency. If the modification was significant, ask the user if they want to review or restructure the entire document. If minor, perform a self-correction/polishing pass autonomously.

## 6. Security & Environment Policy
- **Code Portability Check**: Before any commit, meticulously check for hardcoded absolute paths or variables that depend on a specific local environment.
- **Documentation Paths**: In `.md` files (like those in `./d2r-spec`), paths are acceptable but you MUST use **relative paths** from the project root instead of absolute paths whenever possible.
- **Source Code Variables**: For actual application code, entirely avoid hardcoding paths or sensitive environment data. Always migrate these to `.env` configuration files or appropriate configuration injection mechanisms.
