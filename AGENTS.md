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
| `#X array` | `#X array name 100 float 3;` | ✅ YES (garray gobj; Pd `graph_array` → `glist_add`) |
| `#X scalar` | `#X scalar template 1 2 3;` | ✅ YES (scalar gobj; Pd `glist_scalar` → `glist_add`) |

> **`#X array`/`#X scalar` occupy a connect index.** In Pd's file loader both
> are added to the current canvas's `gl_list`, so they consume an object index
> exactly like an `#X obj`. Skipping them shifts every following connection
> index in that canvas. (This reverses the earlier "array is not an object"
> convention — verified against `src/g_array.c` and `src/g_canvas.c` in the Pd
> source.) In practice arrays usually sit alone in a graph subcanvas with no
> connections, which is why the bug stayed hidden; it still corrupts any
> canvas that mixes an array/scalar with connected objects.

### What does NOT count as an object

| Entry type | Example | Why |
|---|---|---|
| `#N canvas` | `#N canvas 0 22 450 300 12;` | Canvas header |
| `#X connect` | `#X connect 0 0 1 0;` | Connection reference |
| `#X coords` | `#X coords 0 7 16 0 200 140 1 0 0;` | Graph config |
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

### Canvas-scoped addressing (sibling subpatches)

Object indices and connections are **per canvas**, not per depth. In Pd,
`canvas_connect` resolves indices by walking the *current* canvas's `gl_list`,
and every `#N canvas` resets the index counter. Two sibling `pd` subpatches at
the same depth therefore each start at object index 0 — depth alone does not
identify an object.

The parser records a per-entry `canvas_id`. Edit commands select a sibling with
`--canvas N` (the Nth canvas at that depth in document order; default 0 = first
sibling = legacy behaviour). Use the canvas-scoped `Patch` helpers
(`resolve_canvas`, `object_in_canvas`, `object_count_in_canvas`,
`connections_in_canvas`, plus the free `resolve_canvas_id` /
`canvas_ids_at_depth` for raw entry slices) rather than the depth-only
`object_at` / `connections_at_depth`, which merge siblings and are kept only for
legacy callers. `validate` counts objects per `canvas_id` for the same reason.

### Subpatch header forms

A `#N canvas` header has two shapes (see Pd `canvas_new`):
- **Root / abstraction:** `#N canvas X Y W H FONT;` (5 args after `canvas`)
- **Subpatch (subwindow):** `#N canvas X Y W H NAME VIS;` (6 args: name + vis flag)

When emitting a new subpatch inside a patch (the `subpatch` command) use the
6-arg subwindow form and close it with `#X restore X Y pd NAME;`. `extract`
emits the 5-arg form because the extracted file is a *root* canvas.

### External resolution (`deps`)

Pd resolves an unknown object class by trying, per search directory:
compiled externals (`.pd_linux`/`.l_amd64`/`.so`/`.pd_darwin`/`.dll`/…),
loader externals (`.pd_lua`, `.pd_luax`), then abstractions (`.pd`, `.pat`,
and the `name/name.pd` class-in-folder convention). `deps` mirrors this set;
checking only `.pd` reports Lua/compiled externals as false `MISSING`.

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

1. Implement the command in the Rust crate (`src/commands/<command>.rs`, register in
   `src/commands/mod.rs`, add the subcommand to `src/cli.rs`, dispatch in `src/main.rs`)
2. Add integration tests in `tests/test_<command>.rs` (they share `mod integration;`)
3. Create any new fixtures needed
4. Add integration tests to the shell harness (`test_pdtk_integration` function)
5. Run full checks: `make lint && make test` (see [Checking your work](#checking-your-work))

### Modifying the parser

1. **ALWAYS** run the round-trip test on all fixtures before and after
2. Check `expected.json` values still match
3. Run corpus fixtures — these catch real-world regressions
4. Pay special attention to index assignment — off-by-one errors here corrupt every connection in the file
5. Run full checks (see [Checking your work](#checking-your-work))

### Checking your work

Before committing any change:

```sh
# Lint + full test suite (nextest, doctests, shell harness) — same checks CI runs
make lint
make test

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
3. **DO count `#X array` and `#X scalar` as objects** — they are `gobj`s in Pd and take a connect index; not counting them mis-indexes connections in a canvas that mixes an array/scalar with wired objects
4. **Sibling subpatches don't share an index space** — objects inside two same-depth `pd` subpatches both start at index 0; use `--canvas N` (and the canvas-scoped `Patch` helpers) to disambiguate
5. **`\;` is NOT a terminator** — it's an escaped semicolon inside message content. But an *unescaped* `;` terminates an entry **anywhere**, not only at end-of-line: the tokenizer splits on every unescaped `;` (matching Pd's `binbuf_text`), so `a; b;` on one physical line is two entries. A stray unescaped `;` in a message body therefore produces a bare fragment entry (flagged by `validate`).
6. **`#X restore` changes depth BEFORE getting an index** — pop depth first, then assign index at new depth.
   Example: depth-0 has `obj_A` (idx 0), `obj_B` (idx 1), then a subpatch opens. Inside at depth-1: `obj_C` (idx 0), `obj_D` (idx 1). The `#X restore` pops depth 1→0 and receives idx **2** at depth 0. Wrong: assigning it idx 2 at depth 1 corrupts every parent-level connection.
7. **Multi-line entries** — a single entry may span physical lines; continuation lines are joined until an unescaped `;` terminator is reached (Pd inserts no column wrapping, only a newline after each real `;`).
8. **`, f N` at end of an entry** — this is an inline width hint, NOT part of the object class or arguments
9. **`f` as an object class** — `#X obj 50 50 f;` is a float box. Don't confuse with width hints.
10. **Message boxes are senders too** — a `\;`-introduced sub-message targets a named receiver (`#X msg ... \; pitch 60;` sends to `[r pitch]`). The first token after each `\;` is the target; the leading (pre-`\;`) message goes out the box outlet and is NOT a target. Send/receive analysis (`send_receive::collect_sends`) and `rename-send` both handle these via `model::message_send_targets`. Engine/canvas targets (`pd`, `pd-<name>`) are excluded from bus analysis but still renamable. Missing these silently breaks `rename-send`.

