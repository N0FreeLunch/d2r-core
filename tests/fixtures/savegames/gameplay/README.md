# Gameplay Save Games

This folder contains actual gameplay save data for Diablo II Resurrected.

## Naming Standard

All gameplay fixtures must follow this standard:

`[CharacterName]_[QuestContext/Step]_[DetailedDescription].d2s`

- **CharacterName**: Name of the character (e.g., `TESTDRUID`).
- **QuestContext**: Quest or progression point (e.g., `Q1EOB`, `Q6Andariel`).
- **DetailedDescription**: Specific state description (e.g., `Done_PreAkara`).

## Tools

You can use the `fixture_organizer` tool to automatically move and rename new save files:

```bash
cargo run --bin fixture_organizer -- --char TESTDRUID --quest Q1 --desc Done --src ./mysave.d2s
```

This tool reads the difficulty and act information from the file and automatically places it in the `gameplay/{difficulty}/{act}/` path.

## Features

- **Default Character Name**: `TESTDRUID`
- **Item Composition**: As these are files from actual gameplay, they contain a mixture of various items in the inventory and stash.
