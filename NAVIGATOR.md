# D2R-Core Project Navigator

This is the primary index for AI agents (Gemini CLI/Code Assistant) to locate truth, patterns, and verification tools. **Read this first if you are lost.**

## 1. Core Domains & File Map

| Domain | Specification (Truth) | Primary Implementation | Verification Tool |
| :--- | :--- | :--- | :--- |
| **Bitstream / Save Header** | `d2r-spec/discussion/0008-*.md`, `d2r-spec/discussion/0018-*.md` | `src/save.rs` | `src/bin/verify/d2save_map.rs`, `src/bin/verify/d2save_verify.rs` |
| **Item parsing / Decrypt** | `d2r-spec/discussion/0016-*.md` | `src/item.rs` | `src/bin/verify/d2item_inspect.rs` |
| **Inventory / Grid** | `d2r-spec/discussion/0021-*.md` | `src/inventory.rs` | `src/bin/verify/d2save_inventory_check.rs` |
| **Save Verification** | `d2r-spec/discussion/0019-*.md` | - | `src/bin/verify/d2save_verify.rs` |
| **UI / Orchestration** | `d2r-spec/discussion/0010-*.md` | `src/main.rs` | Elm-rs generated types |
| **Workflow / Rules**   | `AGENTS.md`, optional `d2r-spec/AI_WORKFLOW.md`, optional `d2r-spec/AGENTS.md` | optional `d2r-spec/.agents/tasks/*.md`, optional `d2r-spec/.agents/failures/*.md` | - |

## 2. Recent Architectural Decisions (Must Know)
- **Rust + Elm**: Core logic is in Rust; UI is in Elm.
- **Type Safety**: Use `elm-rs` for 1:1 type mapping. No manual JSON types.
- **Verification-First**: Never consider a code change "done" until verified with a tool in `src/bin/verify/`.
- **D2R/DLC Aware**: We prioritize D2R/DLC support over classic LoD logic.

## 3. Git-Aware Context Recovery
When you need to know **why** a specific byte offset or bit width was chosen:
- `git log -p <file>`: Understand the evolution of complex bit-parsing logic.
- `git blame <file>`: Find the specific commit/task that introduced a change.
- `git log --grep="00XX"`: Find all code changes related to a specific specification.
- **Tip**: Our commit messages are in English for core logic, and Korean for specific d2r-spec discussions.

## 4. Important Paths
- **Specs/Discussions**: `./d2r-spec/discussion/` (Historical and design context)
- **Private Overlay**: `./d2r-spec/AGENTS.md` (Optional local-only extension of root agent policy)
- **Private Workflow**: `./d2r-spec/AI_WORKFLOW.md` (Optional local-only workflow detail)
- **Agent Tasks**: `./d2r-spec/.agents/tasks/` (Preferred local-only implementation plans)
- **Failure Reports**: `./d2r-spec/.agents/failures/` (Preferred local-only escalation data)
- **Fixtures**: `./tests/fixtures/savegames/` (Reference binary files)
- **Verification Tools**: `./src/bin/verify/` (Standalone CLI tools for testing specific bytes)
- **Generated Data**: `./src/data/` (Lookups extracted from game files)

## 5. How to Research (Agentic Loop)
1.  **Check `NAVIGATOR.md`**: Find which spec and file are relevant.
2.  **Check Private Overlay If Present**: If `./d2r-spec/AGENTS.md` or `./d2r-spec/AI_WORKFLOW.md` exists locally, apply it as an extension of root policy for private research and planning work.
3.  **Check Private Task Specs If Present**: Prefer `./d2r-spec/.agents/tasks/` for active private execution plans.
4.  **Read Spec**: Always read the corresponding `00XX` spec before coding.
5.  **Git Research**: Use `git log -p` if the "why" behind existing code is unclear.
6.  **Locate Patterns**: Search `src/` for similar implementation patterns.
7.  **Verify**: Identify and run the matching `src/bin/verify/` tool.
8.  **Escalate Correctly**: If the task is `3+ files` or deep logic, refresh the task spec and route full implementation to a stronger secondary model.

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
```
