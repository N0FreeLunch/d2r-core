# d2r-core

A library for reading and writing Diablo II: Resurrected save files.

## Compatibility & Status

- **Target Version**: D2R **Version 3.0+** (`Reign of the Demonologist` DLC, Internal **0x69**)
- **State**: Core parsing is stable for the 3.0+ expansion spec. Full data-integrated formatting is in progress.


## License

This project is licensed under the Apache License, Version 2.0. See the [LICENSE](LICENSE) and [NOTICE](NOTICE) files for details.

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

### Surgical & Archaeological Tools (Bit-Level)

The following tools are designed for deep bitstream analysis and "archaeological" reconstruction of unknown or corrupted item data. **(Core)**

- **`d2item_bit_scan`**: Scans the bitstream for valid item codes using a sliding window.
- **`d2item_bit_width_probe`**: Probes the optimal bit-width (9, 10, or 11) for property IDs.
- **`d2item_bit_peek`**: Peeks at the item header and raw bits at a specific offset.
- **`d2item_bit_dump`**: Dumps bit-level data in a visual matrix (9/10/11 bit rows).
- **`d2item_bit_search`**: Searches for specific bit patterns/values in the bitstream.
- **`d2item_brute_len`**: Identifies property list boundaries by scanning for terminators.
- **`d2item_chunk_verify`**: Aggressively verifies item chunk integrity and roundtrip safety.

