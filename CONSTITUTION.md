# 📜 D2R-Core Constitution (Bootstrap)

> **Status:** `Strict Adherence Required`  
> **Methodology:** `Spec-driven Design (SDD)`  
> **Authority:** Preserve the bootstrap safety floor in this file first; if a local Strategy Hub companion is available through `D2R_SPEC_PATH`, it may provide richer governance notes.

---

## ⚖️ Core Principles

1. **Specification Authority**  
   All development must preserve the public bootstrap safety invariants in this repository and align with any verified technical specifications available in the current environment.

2. **Source of Truth**  
   In core-only environments, local source, fixtures, and verifiers remain valid working truth. If a local `d2r-spec` Strategy Hub is available, it may provide richer companion research and architectural notes.

3. **Compliance**  
   Every contribution must be checked against the verified documentation, fixtures, and verifier surface available for the current environment before merging.

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
