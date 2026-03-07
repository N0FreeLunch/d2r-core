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

---
*Note: This document is intended for AI agents to understand the operational context of this project.*
