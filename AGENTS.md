# Agent Guide — pd-toolkit Test Harness

This document explains the test infrastructure for AI coding agents working on `pdtk`, the Pure Data patch parser/editor.

## Repository Layout

```
pd-toolkit/
├── rustcli.md                      # Full project plan and specification
├── AGENTS.md                       # This file — agent reference
├── tests/
│   ├── run_tests.sh                # Shell test harness (runs without Rust)
│   ├── expected.json               # Expected parse results per fixture
│   ├── fixtures/
│   │   ├── MANIFEST.md             # Fixture inventory with annotations
│   │   ├── handcrafted/            # Hand-written .pd files (28 files)
│   │   ├── corpus/                 # Real patches from sequencer repo (15 files)
│   │   └── abstractions/           # Dependency testing fixtures (3 files)
│   └── (future: integration/ snapshots/ — created by Rust test suite)
└── (future: pd-toolkit/ — Rust crate directory)
```

## Running Tests

### Shell tests (always available, no Rust required)

```sh
# Run all tests
./tests/run_tests.sh

# Quick mode — handcrafted fixtures only
./tests/run_tests.sh --quick

# Corpus tests only
./tests/run_tests.sh --corpus

# Verbose — show passing tests too (failures always show without this flag)
./tests/run_tests.sh --verbose
```

Exit codes: `0` = all pass, `1` = failures exist.

### Rust tests (once pd-toolkit crate exists)

```sh
cd pd-toolkit
cargo test                    # Unit + integration tests
cargo insta test              # Snapshot tests
cargo test -- --ignored       # Long-running property tests
```

### pdtk integration tests via shell harness

The shell harness auto-detects the pdtk binary and runs integration tests:

```sh
# Auto-detect from target/debug or target/release
./tests/run_tests.sh

# Explicit path
PDTK=./target/debug/pdtk ./tests/run_tests.sh
```

## Test Fixtures

### Handcrafted fixtures (`tests/fixtures/handcrafted/`)

28 `.pd` files, each targeting specific parser edge cases. **These are the source of truth for correctness.** See `MANIFEST.md` for the full inventory.

Key fixtures for parser correctness:

| File | What it tests | Why it matters |
|------|--------------|----------------|
| `with_declare.pd` | Standalone `#X declare` is NOT an object | Getting this wrong shifts all connection indices |
| `with_width_hint.pd` | `#X f 38;` after restore is NOT an object | Same — index corruption if counted |
| `with_c_entry.pd` | `#C restore;` non-standard entry | Must not crash or corrupt indices |
| `multiline_obj.pd` | Multi-line message entry | Tokenizer must join lines until `;` |
| `escaped_semicolons.pd` | `\;` inside messages | Must NOT split the entry |
| `float_vs_width.pd` | `f` as object class vs `, f N` width hint | Three different meanings of `f` |
| `nested_subpatch.pd` | `#X restore` gets index at parent depth | Depth stack + index assignment |
| `graph_and_pd_subpatches.pd` | Both subpatch types coexist | Graph restore is also an object |

### Corpus fixtures (`tests/fixtures/corpus/`)

15 files copied from the real sequencer project. These exercise real-world complexity including:
- 1200-line files with deep nesting (`complex_nested_real.pd`)
- Multiple `#X f` width hints (`width_hint_real.pd`)
- `#C restore;` entries (`c_entry_real.pd`)
- Escaped `\;` in messages (`escaped_semicolons_real.pd`)
- Standalone `#X declare` directives (`declare_real.pd`)
- 2800+ toggle objects in a single file (via `complex_nested_real.pd`)

### Expected values (`tests/expected.json`)

Machine-readable expected parse results for every fixture. Includes:
- Object counts per depth
- Connection counts per depth
- Object class at each index
- Connection endpoints
- GUI send/receive field values
- Orphan/display indices
- Error expectations for malformed files

**Use this file when implementing Rust tests.** Load the JSON, iterate fixtures, assert actual == expected.

### Abstraction fixtures (`tests/fixtures/abstractions/`)

For `deps` command testing:
- `used_abs.pd` — referenced by `uses_abstractions.pd`
- `unused_abs.pd` — exists on disk but not referenced
- `uses_abstractions.pd` — references `used_abs` (found) and `missing_abs` (not found)

## Critical PD Format Rules

These are the rules that **must** be followed when creating or modifying `.pd` fixtures. Getting any wrong will produce tests that validate incorrect behavior.

### What counts as an object (gets a 0-based index)

| Entry type | Example | Object? |
|---|---|---|
| `#X obj` | `#X obj 50 50 osc~ 440;` | ✅ YES |
| `#X msg` | `#X msg 50 50 bang;` | ✅ YES |
| `#X text` | `#X text 50 50 comment;` | ✅ YES |
| `#X floatatom` | `#X floatatom 50 50 5 0 0 0 - - -;` | ✅ YES |
| `#X symbolatom` | `#X symbolatom 50 50 10 0 0 0 - - -;` | ✅ YES |
| `#X restore` | `#X restore 50 50 pd name;` | ✅ YES (at parent depth) |

### What does NOT count as an object

| Entry type | Example | Why |
|---|---|---|
| `#N canvas` | `#N canvas 0 22 450 300 12;` | Canvas header |
| `#X connect` | `#X connect 0 0 1 0;` | Connection reference |
| `#X coords` | `#X coords 0 7 16 0 200 140 1 0 0;` | Graph config |
| `#X array` | `#X array name 100 float 3;` | Array definition |
| `#X declare` | `#X declare -path pos_abs;` | Standalone directive |
| `#X f` | `#X f 115;` | Width hint for preceding object |
| `#A` | `#A 0 0 0 0;` | Array data |
| `#C` | `#C restore;` | Non-standard/corrupted |

### `#X restore` depth rule

`#X restore` **closes** the current subpatch depth and is assigned an index at the **parent** depth. The depth stack must be decremented before assigning the index.

### Connection placement

PD writes connections **after** all objects at the same depth. Within a subpatch, the order is:
1. `#N canvas` (opens subpatch)
2. Objects (`#X obj`, `#X msg`, etc.)
3. Connections (`#X connect`)
4. `#X restore` (closes subpatch — becomes object at parent depth)

## Code Style

- Run `cargo fmt` before committing Rust code; fix all `cargo clippy` warnings
- Follow git commit conventions — invoke `/skill:git-conventions` or read `.pi/skills/git-conventions/SKILL.md` before committing
- **Never commit `rustcli.md` or `pythoncli.md`** — these are development planning documents, not part of the codebase
- Do not add module-level doc comments
- Do not add section separator comments in files:
```rust
// DO NOT DO THIS
// ---------------------------------------------------------------------------
// Entry kinds
// ---------------------------------------------------------------------------
```
```sh
// DO NOT DO THIS EITHER
# ─── Structural validation helpers ───────────────────────────────────
```
- Add docstrings for all public functions and types; keep them concise

## Workflow for Making Changes

### Adding a new test fixture

1. Create the `.pd` file in `tests/fixtures/handcrafted/`
2. Add it to `tests/fixtures/MANIFEST.md` with annotations
3. Add expected values to `tests/expected.json`
4. Run `./tests/run_tests.sh` to verify the fixture passes (add `--verbose` to also see passing tests)
5. If the fixture exercises a new edge case, document it in `rustcli.md` §3.0

### Adding a new pdtk command

1. Implement the command in the Rust crate
2. Add integration tests in `tests/integration/test_<command>.rs`
3. Create any new fixtures needed
4. Add integration tests to the shell harness (`test_pdtk_integration` function)
5. Run full checks: `cargo fmt && cargo clippy && cargo test && ./tests/run_tests.sh`

### Modifying the parser

1. **ALWAYS** run the round-trip test on all fixtures before and after
2. Check `expected.json` values still match
3. Run corpus fixtures — these catch real-world regressions
4. Pay special attention to index assignment — off-by-one errors here corrupt every connection in the file
5. Run full checks (see [Checking your work](#checking-your-work))

### Checking your work

Before committing any change:

```sh
# Shell tests must pass
./tests/run_tests.sh

# If Rust code exists
cd pd-toolkit && cargo fmt && cargo clippy && cargo test && cargo doc

# Verify no fixture was accidentally modified
git diff tests/fixtures/
```

## Key Files to Read First

1. **`rustcli.md` §3.0**  — All 11 parser problem areas with solutions and code snippets
2. **`rustcli.md` §3.5**  — Connection renumbering (most dangerous operation)
3. **`tests/fixtures/MANIFEST.md`** — What each fixture tests
4. **`tests/expected.json`** — Source of truth for expected parse results
5. **`rustcli.md` §7** — Full testing strategy including property tests and CI

## Common Pitfalls

1. **Don't count `#X declare` as an object** — it looks like one but standalone declares are invisible to connections
2. **Don't count `#X f N` as an object** — it's a width hint that follows `#X restore`
3. **`\;` is NOT a terminator** — it's an escaped semicolon inside message content
4. **`#X restore` changes depth BEFORE getting an index** — pop depth first, then assign index at new depth.
   Example: depth-0 has `obj_A` (idx 0), `obj_B` (idx 1), then a subpatch opens. Inside at depth-1: `obj_C` (idx 0), `obj_D` (idx 1). The `#X restore` pops depth 1→0 and receives idx **2** at depth 0. Wrong: assigning it idx 2 at depth 1 corrupts every parent-level connection.
5. **Multi-line entries** — continuation lines don't start with `#`. Only lines starting with `#` begin new entries.
6. **`, f N` at end of an entry** — this is an inline width hint, NOT part of the object class or arguments
7. **`f` as an object class** — `#X obj 50 50 f;` is a float box. Don't confuse with width hints.

