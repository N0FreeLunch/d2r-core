# AI Agent Guidelines

This document outlines the operational priorities and constraints for AI agents working on this project.

## 1. Language Policy (Highest Priority)
- **Primary Language**: English is the primary language for this project.
- **Requirement**: All source code, variable names, function names, comments, and project documentation (unless explicitly requested otherwise for specific Korean discussion/analysis files) **must** be written in English.

## 2. Specification-Driven Development
- **Requirement**: Agents must always consult the documents in the `d2r-spec` folder before implementing new features or refactoring core logic.
- **Goal**: Ensure all implementations align strictly with the established specifications and design decisions.

## 3. Split Repository Structure
- **Root Repository (`d2r`)**: Contains the core implementation, logic, and build systems.
- **Specification Repository (`d2r-spec`)**: A separate repository linked as a symbolic link (`d2r/d2r-spec`). It contains design documents, analysis raw data, and development roadmap.
- **Separation**: Implementation and specification are strictly separated into these two repositories.

## 4. Agent Execution & Terminal Context
- **Working Directory**: Be mindful of the terminal context. Run core development commands within `d2r` or `d2r-core`.
- **Symbolic Link Awareness**: `d2r-spec` is a symbolic link. Git operations on the specification itself must be performed within the context of its original repository path, as it is ignored via `.gitignore` in the root repository.

## 5. Scripting & Orchestration Architecture (Strict Rule)
- **Prohibited**: Do NOT write OS-dependent shell scripts (`.ps1`, `.sh`, `.bat`) for orchestrating tests or editing features. Avoid standalone Python or Node.js scripts for file operations.
- **Core Engine**: `Rust` is strictly used for file I/O, pure domain logic parsing, and bit-level assertions.
- **Orchestration & Workflow**: The orchestration of editing logic (e.g. moving items, verifying multiple files) is governed by **Elm** communicating with Rust.
- **Protocol & Type Safety**: Always use **`elm-rs`** to automatically generate direct 1:1 Elm types from Rust structs. Do NOT use multi-step conversions like `typeshare` which require intermediate TypeScript layers.
- **Enforcement**: Any automation or testing workflow created must adhere to this API/Protocol-driven Golden Master TDD strategy as defined in `0020` discussion document.

---
*Note: This document is intended for AI agents to understand the operational context of this project.*
