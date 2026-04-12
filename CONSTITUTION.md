# 📜 D2R-Core Constitution (Bootstrap)

> **Status:** `Strict Adherence Required`  
> **Methodology:** `Spec-driven Design (SDD)`  
> **Authority:** For detailed governance, consult **`../d2r-spec/CONSTITUTION.md`** (Strategy Hub Private Companion)

---

## ⚖️ Core Principles

1. **Specification Authority**  
   All development must strictly align with the technical specifications defined in the [d2r-spec](d2r-spec/) repository.

2. **Source of Truth**  
   The `d2r-spec` subdirectory acts as the definitive source for file formats, research data, and architectural patterns.

3. **Compliance**  
   Every contribution must be validated against the research documentation before merging.

---

## 🔒 Security Policy

> [!CAUTION]
> **Environment Security Whitelisting Required**
> When execution warnings appear for generated binaries (`.exe`), especially during Rust build and execution, they must be whitelisted in Windows Defender or 3rd party security systems, and reputation-based protection (SmartScreen) should be disabled. This is a **prerequisite for operational success** as security blocks prevent the binary-driven verification logic from functioning.

---

## Operational Addendum (2026-03-23)

1. **Directive Precedence**
   If model entry files (`gemini.md`, `CLAUDE.md`) conflict with `AGENTS.md`, the `AGENTS.md` safety constraints take precedence.

2. **Constitution-Safe Update Rule**
   Constitution-level files must be updated via minimal patching only:
   - Preserve existing rules by default.
   - Edit only direct contradictions.
   - Append clarifications when no contradiction exists.

3. **Non-Negotiable Safety Invariants**
   - No automatic `git push` without explicit user command.
   - Data boundary integrity (`d2r-core` vs `d2r-data`) is mandatory.
   - Golden-master/fixture truth is the final arbiter for parser or serializer behavior.
