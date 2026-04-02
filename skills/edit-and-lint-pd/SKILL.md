---
name: edit-and-lint-pd
description: How to read, edit, and auto-format Pure Data (.pd) patch files. Uses pdtk — a CLI that handles object indexing, connection renumbering, and validation automatically. Use when creating, modifying, debugging, or reformatting .pd files.
---

# Editing and Formatting Pure Data Patches

**Tool:** `pdtk` must be on your PATH, or set `PD=path/to/pdtk` and use `$PD` throughout.

## Workflow

```
1. pdtk list file.pd               # understand structure
2. read file.pd                     # examine full content
3. pdtk <command> ...               # make changes
4. pdtk validate file.pd            # verify connections valid
5. pdtk format file.pd --in-place   # clean up layout (optional)
6. pdtk validate file.pd            # final check
```

## Depth convention

Depths are **0-based**: `--depth 0` = top-level canvas, `--depth 1` = first subpatch level.

## Command reference

Detailed usage for each command group:

- [Inspection commands](./inspection.md) — parse, list, validate, lint, stats, connections, arrays
- [Search & analysis commands](./search.md) — search, find-orphans, find-displays, trace, diff, deps
- [Editing commands](./editing.md) — insert, delete, modify, connect, disconnect, renumber, rename-send
- [Layout commands](./layout.md) — format
- [Subpatch & batch commands](./subpatch-batch.md) — extract, batch

## .pd file structure

```
#N canvas X Y W H fontsize;          # canvas header (NOT an object)
#X obj X Y class args...;            # object 0
#X obj X Y class args...;            # object 1
#X msg X Y content;                  # object 2 (message box)
#X text X Y comment;                 # object 3 (comment — IS an object)
#X floatatom X Y W 0 0 0 - - -;     # object 4 (number display)
#X connect 0 0 2 0;                  # connections — always after all objects
#X connect 1 0 2 0;
```

### What counts as an object (gets a 0-based index)

✅ `#X obj`, `#X msg`, `#X text`, `#X floatatom`, `#X symbolatom`, `#X restore`
❌ `#N canvas`, `#X connect`, `#X coords`, `#X array`, `#X declare`, `#X f N`, `#A`

### `#X restore` is an object in the PARENT canvas

```
#X obj 50 50 inlet;             # depth-0 index 0
#N canvas 0 0 450 300 sub 0;   # opens depth 1
  #X obj 50 50 + 1;            # depth-1 index 0
  #X obj 50 100 outlet;        # depth-1 index 1
  #X connect 0 0 1 0;
#X restore 50 100 pd sub;      # depth-0 index 1 (closes depth 1)
#X obj 50 150 print;           # depth-0 index 2
#X connect 0 0 1 0;            # inlet(0) → sub(1)
#X connect 1 0 2 0;            # sub(1) → print(2)
```

### Connection syntax

```
#X connect SOURCE_INDEX SOURCE_OUTLET DEST_INDEX DEST_INLET;
```

All indices 0-based. Connections appear after all objects at the same depth.

## Getting more detail

When uncertain about a flag or its exact syntax, **run the tool rather than guess**:

```bash
pdtk --help                  # list all commands
pdtk <command> --help        # flags and examples for one command
pdtk insert --help
pdtk format --help
```

Man pages (if installed) give the same information in long form:

```bash
man pdtk                     # top-level (if system-installed)
man pdtk-insert

# If installed locally (e.g. .tools/man/man1/):
man -l .tools/man/man1/pdtk-insert.1
man -l .tools/man/man1/pdtk.1
```

`--help` is always the authoritative source — it is generated directly from the
binary's command definitions and is guaranteed to match what the tool accepts.

## Critical rule

**Always use `pdtk insert` and `pdtk delete` — never hand-edit connections after structural changes.** The tool handles renumbering and validates before writing.
