This is the public bootstrap index for AI agents to locate implementation files, verification tools, and the private overlay entrypoints. **Read this first if you are lost.**

## 🌐 Tripartite Navigation Map

| Repository | Role | Navigator |
| :--- | :--- | :--- |
| **`d2r-core`** | **Implementation** (Public Logic) | [NAVIGATOR.md](./NAVIGATOR.md) |
| **`d2r-data`** | **Game Data** (Extracted Tables) | [NAVIGATOR.md](./d2r-data/NAVIGATOR.md) |
| **`d2r-spec`** | **Specification** (Private Overlay) | [NAVIGATOR.md](./d2r-spec/NAVIGATOR.md) |

## ⚡ Quick Access: Active Components

| Component | Implementation | Specification / Context | Verification |
| :--- | :--- | :--- | :--- |
| **Parsing Engine** | `src/item.rs`, `src/save.rs` | `discussion/0016`, `0023` | `d2item_inspect` |
| **Validation Engine** | `src/engine/validation.rs` | `discussion/0034` | `item_validation_test` |
| **Interpretation** | `src/engine/formatter.rs` | `discussion/0033`, `0034` | `option_rendering_test` |
| **Inventory Grid** | `src/inventory.rs` | `discussion/0021` | `d2save_inventory_check`|
| **Game Data Gateway**| `src/data/mod.rs` | `discussion/0035` | `cargo check` |

## 1. Core Domains & File Map

| Domain | Specification (Truth) | Primary Implementation | Verification Tool |
| :--- | :--- | :--- | :--- |
| **Bitstream / Save Header** | `d2r-spec/NAVIGATOR.md` -> Bitstream / Save Header domain (private overlay, if present) | `src/save.rs` | `src/bin/verify/d2save_map.rs`, `src/bin/verify/d2save_verify.rs` |
| **Item parsing / Decrypt** | `d2r-spec/NAVIGATOR.md` -> Item parsing / Decrypt domain (private overlay, if present) | `src/item.rs` | `src/bin/verify/d2item_inspect.rs` |
| **Inventory / Grid** | `d2r-spec/NAVIGATOR.md` -> Inventory / Grid domain (private overlay, if present) | `src/inventory.rs` | `src/bin/verify/d2save_inventory_check.rs` |
| **Status (Attrs/Skills)** | `d2r-spec/NAVIGATOR.md` -> Status (Attrs/Skills) domain (private overlay, if present) | `src/save.rs` | `src/bin/d2save_status_inspect.rs` |
| **Save Verification** | `d2r-spec/NAVIGATOR.md` -> Save Verification domain (private overlay, if present) | - | `src/bin/verify/d2save_verify.rs` |
| **UI / Orchestration** | `d2r-spec/NAVIGATOR.md` -> UI / Orchestration domain (private overlay, if present) | `src/main.rs` | Elm-rs generated types |
| **Game Data Access / Copyright Boundary** | `d2r-spec/discussion/0035-data-separation-and-copyright-strategy.md` (private overlay, if present) | `src/data/mod.rs`, `d2r-data/` | [d2r-data/NAVIGATOR.md](./d2r-data/NAVIGATOR.md) |
| **Item Validation**      | `d2r-spec/discussion/0034-item-option-interpretation.md` | `src/engine/validation.rs` | `tests/item_validation_test.rs` |
| **Environment / Paths** | `d2r-spec/discussion/0036-environment-path-normalization.md` | `.env` | `tests/common.rs` |
| **Workflow / Rules**   | `AGENTS.md` (public bootstrap), `d2r-spec/AGENTS.md`, `d2r-spec/AI_WORKFLOW.md` (private overlay) | `d2r-spec/.agents/tasks/` preferred, `./.agents/tasks/` public-safe fallback | - |

## 2. Recent Architectural Decisions (Must Know)
- **Rust + Elm**: Core logic is in Rust; UI is in Elm.
- **Type Safety**: Use `elm-rs` for 1:1 type mapping. No manual JSON types.
- **Verification-First**: Never consider a code change "done" until verified with a tool in `src/bin/verify/`.
- **D2R/DLC Aware**: We prioritize D2R/DLC support over classic LoD logic.
- **External Data Boundary**: Extracted tables are maintained in `d2r-data/` (root link to sibling repo). In `d2r-core`, only `src/data/mod.rs` should bridge into that data.
- **Environment First**: All paths MUST be retrieved via `tests/common.rs` or environment variables (Source of Truth: `.env`). Do not hardcode relative paths like `../../d2r-data`.

## 3. Git-Aware Context Recovery
When you need to know **why** a specific byte offset or bit width was chosen:
- `git log -p <file>`: Understand the evolution of complex bit-parsing logic.
- `git blame <file>`: Find the specific commit/task that introduced a change.
- `git log --grep="00XX"`: Find all code changes related to a specific specification.
- **Tip**: Our commit messages are in English for core logic, and Korean for specific d2r-spec discussions.

## 4. Important Paths
- **Specs/Discussions**: `./d2r-spec/discussion/` (Private design context and internal reasoning, if the local overlay exists)
- **Private Navigator**: `./d2r-spec/NAVIGATOR.md` (Private domain-to-research map)
- **Private Overlay**: `./d2r-spec/AGENTS.md` (Private extension for internal reasoning and workflow)
- **Private Workflow**: `./d2r-spec/AI_WORKFLOW.md` (Private operational workflow, if present)
- **Agent Tasks**: `./d2r-spec/.agents/tasks/` preferred, `./.agents/tasks/` only as a sanitized public-safe fallback
- **Fixtures**: `./tests/fixtures/savegames/` (Reference binary files)
- **Verification Tools**: `./src/bin/verify/` (Standalone CLI tools for testing)
- **Data Gateway (Core)**: `./src/data/mod.rs` (thin `#[path]` gateway into external data repo)
- **External Game Data Repo**: `./d2r-data/` (symlink to `../d2r-data`; extracted tables stay outside `d2r-core` history)

## 5. How to Research (Agentic Loop)
1.  **Check `NAVIGATOR.md`**: Find the logical domain and matching implementation file.
2.  **Locate Specification**: If `d2r-spec/NAVIGATOR.md` exists, find the matching internal document for that domain and continue into the private overlay.
3.  **Check Private Task Specs**: Prefer `d2r-spec/.agents/tasks/` for active execution plans; use `./.agents/tasks/` only for sanitized public-safe fallbacks.
4.  **Read Spec**: Always read the corresponding domain specification before coding.
5.  **Git Research**: Use `git log -p` if the "why" behind existing code is unclear.
6.  **Locate Patterns**: Search `src/` for similar implementation patterns.
7.  **Verify**: Identify and run the matching `src/bin/verify/` tool.
8.  **Escalate Correctly**: If the task is `3+ files` or deep logic, refresh the task spec and route to a stronger model.
9.  **Check Data Boundary**: If requested changes involve extracted game tables/assets, route that scope to `d2r-data` planning and keep `d2r-core` edits limited to gateway/integration behavior.

## 6. Verification Tool Catalog

| Tool Name | Scope | Description & Primary Usage |
| :--- | :--- | :--- |
| `d2save_verify` | Save | Validates checksum, file size, and basic JM marker structure. |
| `d2save_map` | Save | Dumps the memory map of a `.d2s` file (JM offsets, item counts). |
| `d2save_diff` | Save | Byte-level diff between two saves (header vs item section). |
| `d2save_item_diff`| Save | **Crucial**: Compares only the item bitstream, masking header noise. |
| `d2item_inspect` | Item | Decomposes a `.d2i` or `.d2s` item into its bit-fields and props. |
| `d2item_extract` | Item | Extracts a raw item bit-payload from a save into a `.d2i` file. |
| `d2save_inject` | Item | Injects a raw `.d2i` item into a specific save file. |
| **`d2save_status_inspect`**| Status | **New**: Dumps attribute/skill bit-fields and logical values. |
| **`dump_character`** | Save | **Comprehensive**: Dumps full character status, skills, and item map. |
| `d2save_inventory_check`| Logic | Verifies inventory grid integrity (no overlaps, valid coordinates). |

### 🚀 Common Verification Commands

```powershell
# 1. Verify save checksum/magic (after saving a file)
cargo run --bin d2save_verify -- tests/fixtures/savegames/modified/generated.d2s

# 2. Compare item data only (ignoring login metadata/timestamp changes)
cargo run --bin d2save_item_diff -- actual.d2s expected.d2s

# 3. Inspect a specific item's bit-fields
cargo run --bin d2item_inspect -- tests/fixtures/savegames/original/tsc_real.d2i

# 4. Check for inventory grid collisions
cargo run --bin d2save_inventory_check -- path/to/save.d2s

# 5. Inspect character attributes and skills
cargo run --bin d2save_status_inspect -- path/to/save.d2s
```
