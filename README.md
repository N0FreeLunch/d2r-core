# d2r-core

A library for reading and writing Diablo II: Resurrected save files.

## Compatibility & Status

- **Target Version**: D2R **Version 3.0+** (`Reign of the Demonologist` DLC, Internal **0x69**)
- **Retro-Archaeology**: Support for **Alpha v105 (0.05)** save files and forensic bitstream reconstruction.
- **State**: Core parsing is stable for 3.0+ and Alpha v105. Data-integrated formatting is ongoing.


## License

This project is licensed under the **PolyForm Noncommercial License 1.0.0**. This means the software is free for personal and noncommercial use, but commercial use is strictly prohibited. 

**Rationale**: This noncommercial restriction is established to prevent unauthorized commercial exploitation of reverse-engineered game data structures and to ensure the project remains a tool for research and personal use only.

See the [LICENSE](LICENSE) file for the full license text and [NOTICE](NOTICE) for trademark disclaimers.

### Disclaimer
This project is not affiliated with, authorized by, or endorsed by Blizzard Entertainment. Diablo II: Resurrected is a trademark of Blizzard Entertainment. This repository contains only original source code for the parsing engine and does not distribute any copyrighted game assets.

---

## Transparency & Data Boundary

This project enforces a strict **Data Boundary** between the parsing engine's structural logic and the game's proprietary content.

1.  **Structural Disclosure (The Schema)**: We expose the **Rust structs and parsing logic** to prove the engine's integrity and ensure "No Malice." By revealing the *format* of the data, we provide full transparency into how the editor operates without distributing the *content*.
2.  **No Obligation for Data Content**: This repository provides the "vessel" (the code), but is under no obligation to disclose, and does not provide, the actual internal game data or extracted tables. Actual game values are treated as external dependencies to be handled privately by the user.

This approach ensures that while the technical mechanics are open for research and verification, the proprietary assets of the game creators are strictly respected and excluded from the public domain.

### Character Status Editing
Modify core character attributes and skills with full integrity validation.
- **Level & Experience**: Patch character level (1-99) and synchronize header/stat values.
- **Attributes**: Surgical modification of Strength, Dexterity, Vitality, Energy, and Stat points.
- **Skills**: Edit skill point distribution for all character classes.
- **Integrity**: Automatically recalculates checksums and file sizes after any status change.

### Inventory & Item Editing
A comprehensive suite for managing items and storage.
- **Inventory Mapping**: Classify and map items across Inventory, Stash (Personal/Shared), Horadric Cube, and Equipment.
- **Item Modification**: Modify item flags (Ethereal, Socketed, Personalize), IDs, and raw bit-properties.
- **Socketing Engine**: Manage socketed items and their children with recursive bitstream reconstruction.
- **Huffman Engine**: Full support for original Huffman compression of item codes.

### Progression & World Editing
Unlock world features and progression markers.
- **Waypoints**: Activate or deactivate waypoints across all acts and difficulties (Normal, NM, Hell).
- **Quests**: Modify quest completion flags and semantic triggers (e.g., unlocking the Act 3 Durance gate).
- **NPC Status**: Track and modify NPC interaction flags and world state markers.

### Core Parsing & Reconstruction
Standalone features that do not require external game data assets.
- **Save Integrity**: Validate magic numbers, calculate CRC32 checksums, and finalize binary layouts.
- **Bitstream Engine**: Advanced bit-level reading/writing for variable-width Diablo II bitstreams.
- **Section Splicing**: Dynamically rebuild save sections (`gf`, `if`, `JM`) while preserving unknown data anchors.

### Data-Integrated Features (Requires `d2r-data`)
These features use the sibling `d2r-data` repository to provide semantic context.
- **Human-Readable Names**: Resolve internal codes and affix IDs to localized strings.
- **Advanced Stat Resolution**: Interpret complex item properties using game-specific cost tables.
- **Legitimacy Verification**: Validate item base stats, runeword eligibility, and level requirements.

## Verification Tools

The following CLI tools are provided for inspecting and verifying save data. Labels indicate if they require external data.

- `d2save_verify`: Verifies integrity and checksums. **(Core)**
- `d2save_map`: Displays the high-level section map of a save. **(Core)**
- `d2save_diff`: Compares binary/structural differences between saves. **(Core)**
- `d2item_inspect`: Inspects item properties and stats. **(Data Required)**
- `d2item_blob_inspect`: Analyzes raw item bitstreams. **(Core)**
- `d2save_inject`: Injects raw data or modifications into saves. **(Core)**
- `d2item_extract`: Extracts specific item blobs from saves. **(Core)**
- `d2save_item_diff`: Diffs item collections between saves. **(Core)**
- `d2save_grid`: Visualizes the inventory/stash grid layout. **(Core)**
- `d2save_hex_tail`: Displays unknown trailing data in hex format. **(Core)**
- `d2save_inventory_check`: Audits inventory structure for corruption. **(Core)**
- `d2save_bit_analyze`: Deep analysis of bit-level alignment. **(Core)**
- `dump_flags`: Dumps internal status flags. **(Core)**
- `d2save_status_inspect`: Inspects character status (Level, Attributes, Skills). **(Core/Data Mixed)**
- `d2save_status_diff`: Diffs character status between two saves. **(Core/Data Mixed)**
- `dump_character`: Comprehensive dump of character status and item maps. **(Core/Data Mixed)**

### Surgical & Archaeological Tools (Bit-Level)

The following tools are designed for deep bitstream analysis and "archaeological" reconstruction of unknown or corrupted item data. **(Core)**

- **`d2item_bit_diff`**: Visual pairwise alignment of two items. Uses dynamic programming to find gaps/desyncs. **(Core)**
- **`d2item_msa_analyser`**: Multiple Sequence Alignment. Finds common bit-patterns across multiple items. **(Core)**
- **`d2item_oracle_mapper`**: (The Oracle) Performs structural alignment mapping to infer property bit-widths. **(Core)**
- **`d2item_forensic_scan`**: Scans for JM markers and validates Alpha v105 structural anchors. **(Core)**
- **`d2item_v5_peek`**: Specialized forensic viewer for Alpha v105 (0.05) item headers and properties. **(Core)**
- **`d2item_alpha_scavenger`**: Scavenges unknown bitstreams for potential Alpha item patterns. **(Core)**
- **`d2item_bit_align`**: Aligns bitstreams to byte boundaries using multiple strategies. **(Core)**
- **`d2item_structural_fuzzer`**: Fuzzes bitstreams to identify structural dependencies between fields. **(Core)**
- **`d2item_chunk_verify`**: Aggressively verifies item chunk integrity and roundtrip safety. **(Core)**
- **`d2item_brute_len`**: Identifies property list boundaries by scanning for terminators. **(Core)**
- **`d2item_bit_peek`**: Peeks at the item header and raw bits at a specific offset. **(Core)**
- **`d2item_bit_width_probe`**: Probes the optimal bit-width (9, 10, or 11) for property IDs. **(Core)**
- **`d2item_bit_scan`**: Scans the bitstream for valid item codes using a sliding window. **(Core)**

### Experimental Utilities & Examples

- **`v105_unlocker`**: (Example) Unlocks all quests/waypoints for Alpha v105 save files.
- **`v105_quest_semantic_check`**: (Example) Validates quest state consistency for Alpha v105.

