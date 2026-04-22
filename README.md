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

## Library Features

The library is designed with a strict **Data Boundary** to ensure copyright safety. It distinguishes between core parsing logic and game data integration.

### Core Features (Autonomous)
These features work standalone and do not require external game data assets. They rely on original reverse-engineered logic and hardcoded constants (e.g., Huffman tables).

- **Save File Integrity**: Parse and write `.d2s` headers, calculate/fix checksums, and finalize file sizes.
- **Section Parsing**: Map and extract core save sections (`gf` attributes, `if` skills, `JM` items).
- **Bitstream Engine**: Advanced bit-level reading/writing utilities for Diablo II's variable-width bitstreams.
- **Huffman Compression**: Full implementation of the original item-code Huffman compression algorithm.
- **Item Basic Parsing**: Extract item IDs, flags (Ethereal, Socketed, Runeword), position, and raw properties.
- **Character Management**: Modify basic character status such as Level, Quest flags, and Skill points.
- **Inventory Mapping**: Classify and map items to their respective slots (Inventory, Stash, Cube, Equipment).

### Data-Integrated Features (Requires `d2r-data`)
These features require the sibling `d2r-data` repository (extracted game tables) to provide context and human-readable formatting.

- **Human-Readable Names**: Resolve internal item codes and affix IDs to localized strings.
- **Advanced Stat Resolution**: Interpret complex item properties using game-specific cost tables.
- **Legitimacy Verification**: Validate item base stats, runeword eligibility, and level requirements.
- **Full Formatting**: Generate "item tooltip" style text representations of parsed item data.

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

