# pdtk — Pure Data Toolkit

[![crates.io](https://img.shields.io/crates/v/pdtk.svg)](https://crates.io/crates/pdtk)
[![docs.rs](https://img.shields.io/docsrs/pdtk)](https://docs.rs/pdtk)
[![license](https://img.shields.io/crates/l/pdtk.svg)](https://github.com/ModularDevicesMalleableAudio/pd-toolkit/blob/main/LICENSE)

A safe, fast command-line tool for parsing, inspecting, editing, validating,
and auto-formatting Pure Data (`.pd`) patch files — without breaking
connections.

Every mutating command validates the result before writing. Mutations that
would produce an invalid patch are refused, and the source file is never
modified unless explicitly requested with `--in-place`.

---

## Quick start

```sh
# Create a new blank patch
pdtk new      blank.pd           # #N canvas 0 50 450 300 12; (Linux defaults)
pdtk new      blank.pd --width 800 --height 600 --font 10
pdtk new                         # write to stdout

# Inspect a patch
pdtk parse    patch.pd           # object count, depth, warnings
pdtk list     patch.pd           # [depth:index] class args for every object
pdtk validate patch.pd           # check all connection indices are in range
pdtk validate patch.pd --strict  # also warn on duplicate connections
pdtk stats    patch.pd           # fan-in, fan-out, class histogram

# Find things
pdtk search   patch.pd --type route             # find all route objects
pdtk search   patch.pd --type send --text "clk*"  # sends whose name starts with clk
pdtk search   src/ --regex --text "trig_\\d+"     # regex across a tree
pdtk find-orphans  patch.pd                     # objects with no connections
pdtk find-displays patch.pd                     # connected debug number boxes
pdtk deps     patch.pd --missing                # abstractions not found on disk
pdtk deps     patch.pd --buses                  # send/receive name pairs by namespace
pdtk deps     src/  --recursive --buses         # cross-file unsatisfied bus contracts
pdtk trace    patch.pd --from 0                 # what does object 0 connect to?
pdtk trace    patch.pd --from 0 --show-bus-hops # also follow s/r and s~/r~ pairs
pdtk diff     old.pd new.pd --ignore-coords     # what changed (ignoring layout)?

# Lint (combined structural + opt-in heuristics)
pdtk lint     patch.pd                                       # validate + overlap check
pdtk lint     patch.pd --send-receive --fan-out --dsp-loop   # all heuristics

# Edit safely
pdtk insert   patch.pd --depth 0 --index 2 --entry '#X obj 50 75 bang;' --in-place
pdtk delete   patch.pd --depth 0 --index 5 --in-place
pdtk delete   patch.pd --depth 1 --subpatch --in-place         # remove whole subpatch
pdtk modify   patch.pd --depth 0 --index 3 --text 'route 1 2 3' --in-place
pdtk connect  patch.pd --depth 0 --src 0 --outlet 0 --dst 2 --inlet 0 --in-place
pdtk rename-send patch.pd --from clock_main --to clock_renamed --in-place

# Layout
pdtk format   patch.pd --in-place              # auto-reposition objects
pdtk format   patch.pd --grid 20 --hpad 15 --margin 30 --dry-run

# Subpatch operations
pdtk extract  patch.pd --depth 1 --output my_abs.pd --in-place

# Batch
pdtk batch    src/ validate                                   # validate every .pd
pdtk batch    src/ --continue-on-error --json validate        # CI-friendly
pdtk batch    src/ --glob 'sequencer/**/*.pd' format --in-place
```

---

## Commands

| Category | Command | Description |
|---|---|---|
| **Inspection** | `parse` | Object count, connection count, depth, warnings |
| | `list` | List every indexed object with address and class |
| | `validate` | Check connection index ranges and canvas balance. Warns on out-of-range inlets/outlets, including arity derived from sibling `.pd` abstractions (counts their top-level `inlet`/`outlet` objects), and on data-structure inconsistencies (a `#X scalar` with no matching `#N struct`, or a scalar/template field-count mismatch). `--strict` also warns on duplicate connections |
| | `lint` | Validate + layout overlap detection. Opt-in heuristics: `--send-receive`, `--fan-out`, `--dsp-loop` (see [Lint checks](#lint-checks)) |
| | `stats` | Per-file metrics: fan-in/out, class histogram, orphans. Aggregates across all files in directory mode |
| | `connections` | List all patch cords to/from one object |
| | `arrays` | List all PD arrays — classic `#X array` and `array define` — with name, size, options, duplicate detection. `--kind classic\|define\|all`, `--templates include\|exclude\|only`, `--schema 1\|2` |
| | `structs` | List data-structure templates (`#N struct`) with typed fields and scalars (`#X scalar`) with template + value count. Flags scalars whose template is undefined. File or directory, `--json` |
| **Search** | `search` | Find objects by class (`--type`) and/or text (`--text`, glob by default, `--regex` for regex). `--case-sensitive`, `--depth` |
| | `find-orphans` | Objects with zero connections. `--delete --in-place` removes them; `--include-comments` includes `#X text` |
| | `find-displays` | Connected debug display widgets (floatatom/symbolatom/nbx/vu). `--include-unconnected`, `--include-labels`, `--delete` |
| | `trace` | BFS forward trace or path-find. `--show-bus-hops` also follows matching `s`/`r`, `s~`/`r~`, `throw~`/`catch~` pairs within each canvas, respecting the three bus namespaces (see [Send/receive buses](#sendreceive-buses)) |
| | `diff` | Structural diff (objects added/removed/modified, cords). `--ignore-coords` is the pairing for `format` diffs |
| | `deps` | Abstraction dependency list. A class covered by a declared library (`#X declare -lib`/`-stdlib`, or ELSE `[import]`) is reported `unresolved (declared lib: …)` instead of `MISSING` and excluded from `--missing`. `--missing`, `--recursive`, `--search-path DIR` (repeatable fallback), `--pd-path` (append common external locations), `--buses` (bus pairs by namespace; with `--recursive` reports unsatisfied cross-file contracts), `--per-file` (don't merge bus names across files) |
| **Creation** | `new` | Create a blank `.pd` patch. Defaults match PD's `File > New`: 450×300, font 12, y=22 (macOS) / y=50 (Linux) |
| **Editing** | `insert` | Insert object, renumber connections automatically |
| | `delete` | Delete an object (`--index`) or an entire subpatch (`--subpatch`); cords are removed and remaining ones renumbered |
| | `modify` | Change class/args in place, preserving index and connections. Auto-escapes `\$N`/`\;`/`\,` in user input |
| | `connect` | Add a patch cord (refuses duplicates and out-of-range) |
| | `disconnect` | Remove a specific patch cord |
| | `renumber` | Manually shift connection indices by a delta |
| | `rename-send` | Rename s/r pairs atomically across files, including GUI fields and named send targets inside message boxes (`\; name ...`). `--dry-run`, `--force` (override target-exists check) |
| **Layout** | `format` | Auto-reposition objects (connections byte-identical). `--grid`, `--hpad`, `--margin`, `--depth`, `--dry-run` |
| **Subpatch** | `extract` | Extract subpatch into standalone abstraction. Inlet/outlet count inferred from connections crossing the boundary |
| **Utilities** | `batch` | Apply any command recursively across `.pd` files. `--glob`, `--continue-on-error`, `--dry-run` |
| | `completions` | Generate shell tab-completion scripts |

Almost every command accepts `--json` for machine-readable output, and most
non-mutating commands accept `--output PATH` to write to a file instead of
stdout. See `pdtk <cmd> --help` for the full per-command flag set; every
command also has a man page (see [Man pages](#man-pages)).

---

## Send/receive buses

pdtk models PD's three disjoint bus namespaces — they share names but never
route to each other at runtime:

| Namespace   | Senders                                      | Receivers              |
|-------------|----------------------------------------------|------------------------|
| `control`   | `s` / `send`, GUI send fields, message-box `\; name` targets | `r` / `receive`        |
| `signal`    | `s~` / `send~`                               | `r~` / `receive~`      |
| `audio_sum` | `throw~` (sums into one `catch~`)            | `catch~`               |

`trace --show-bus-hops` and `deps --buses` follow bus connections respecting
this split. A `[s foo]` and `[s~ foo]` are never reported as connected.

Message boxes drive named receivers via PD's `\;`-send idiom
(`#X msg ... \; pitch 60;` sends `60` to `[r pitch]`). These count as control
senders in bus analysis and are rewritten by `rename-send`. Engine/canvas
targets (`pd`, `pd-<name>`) are excluded to avoid false orphans.

```
$ pdtk trace patch.pd --from 0 --show-bus-hops
Forward trace from index 0 at depth 0:
  hop 1: [index:1] #X obj 50 100 s foo; (via outlet 0 → inlet 0 from index 0)
  hop 2: [index:2] #X obj 50 200 r foo; (via bus "foo" (control) from index 1)
  hop 3: [index:3] #X obj 50 250 print downstream; (via outlet 0 → inlet 0 from index 2)
```

Bus matching is **per-canvas**: sibling subpatches at the same depth get
their own namespaces, matching PD's runtime scoping.

Names beginning with `$0-` are flagged `scope_warning: dollar-zero-scoped`
in the output. `$0` resolves to a per-instance canvas ID at runtime, which
static analysis cannot follow — so cross-instance matches are surfaced but
marked as potentially false positives.

For project-wide bus auditing:

```sh
pdtk deps src/ --recursive --buses --json | jq '.unsatisfied'
```

reports buses that an abstraction expects but no caller provides (or
vice versa). Abstraction bus names containing `$1`..`$9` are realized against
each call site's arguments (`[looper voice3]` realizes `[r $1-clock]` to
`voice3-clock`) before matching the caller's sends/receives; `$0-` names stay
instance-scoped and are not matched cross-file.

---

## Lint checks

`lint` always runs structural validation (same as `validate`) plus layout
overlap detection. Three further heuristics are opt-in:

| Flag | What it reports | Limitations |
|---|---|---|
| `--send-receive` | Orphan sends (`[s foo]` with no `[r foo]`), dead receives, broadcast receives (multiple `[r foo]` for one name) | Per-canvas scoping; respects bus namespaces |
| `--fan-out` | Control-rate outlets feeding 2+ destinations without a `[trigger]` — message order is undefined | Heuristic: skips sources whose class ends in `~`. Mixed-rate objects (e.g. `snapshot~`) may be false negatives |
| `--dsp-loop` | Static DSP feedback cycles in the signal graph | Per-canvas only — does **not** see through `inlet~`/`outlet~`, abstractions, `pd~`, or `clone` |

All three are informational: they produce `STYLE:` lines but do not change
the exit code. Only structural errors fail lint.

---

## Safety guarantees

- **`format`** only modifies X/Y coordinate fields — an internal assertion
  refuses to write if any connection line was altered.
- **All mutating commands** run `validate` on the result before writing. A
  failed validation prevents any file modification.
- **`--in-place`** is never the default. Stdout is the default output for
  mutation commands.
- **`--backup`** writes `<file>.bak` before overwriting.
- **`modify`, `rename-send`, and `extract`** apply PD's file-level escaping
  (`\$N`, `\;`, `\,`) to user input automatically — pass the unescaped form
  and the on-disk file will match what PD itself would save.

---

## Examples

### Rename a send/receive pair across an entire project

You renamed a clock bus and need every `s clock_main` / `r clock_main` in every
patch — including GUI send/receive fields on toggles and number boxes — updated
atomically:

```sh
pdtk rename-send src/ --from clock_main --to clock_v2 --dry-run
pdtk rename-send src/ --from clock_main --to clock_v2 --in-place --backup
```

`--dry-run` prints every line that would change. `--backup` writes `.bak` files
before overwriting so you can diff or roll back.

### Replace an object without disturbing connections

Swap a `route` for a `route 1 2 3` at depth 0, index 3. The object index and
all patch cords are preserved:

```sh
pdtk modify patch.pd --depth 0 --index 3 --text 'route 1 2 3' --in-place
```

If the new text would leave an existing outlet connection out of range, pdtk
prints a warning but still writes (the connection may need removing separately).

### Run all lint heuristics at once

```sh
pdtk lint patch.pd --send-receive --fan-out --dsp-loop
```

Reports orphan/dead/broadcast buses, untrigger'd control fan-outs, and DSP
feedback cycles alongside the standard structural and overlap checks.

### Trace a bus across a patch

```sh
pdtk trace patch.pd --from 0 --show-bus-hops --to 42
```

Finds the shortest path from object 0 to object 42, following both wires
and bus hops. With `--json`, every step carries `hop_kind` (`wire` or `bus`),
`bus_kind`, `bus_name`, and any `scope_warning`.

### Check a project before deploying to hardware

Validate every patch, list missing abstractions, and report unsatisfied
bus contracts:

```sh
pdtk batch src/ --continue-on-error validate
pdtk deps  src/main.pd --missing
pdtk deps  src/ --recursive --buses --json
```

All three exit non-zero on problems, so they drop cleanly into a `Makefile`
or CI step.

### Diff two patches ignoring auto-format changes

```sh
pdtk format old.pd --output formatted.pd
pdtk diff   formatted.pd new.pd --ignore-coords
```

`--ignore-coords` suppresses coordinate-only changes — essential when
comparing a `format`ted file against its source.

### Extract a subpatch into a reusable abstraction

Pull depth-1 out of `sequencer.pd` into its own file. pdtk infers the required
inlet/outlet count from the existing connections, writes the new file, and
replaces the subpatch in the source with an object box:

```sh
pdtk extract sequencer.pd --depth 1 --output step_counter.pd --in-place
```

---

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Validation / lint structural errors |
| 2 | Parse / usage errors |
| 3 | I/O errors |

`lint` returns 0 even when `STYLE:` warnings are present — only structural
errors (the same ones `validate` catches) yield exit 1. `validate --strict`
treats duplicate connections as warnings, not errors, so they also do not
change the exit code.

---

## Install

### From crates.io

```sh
cargo install pdtk
```

### Prebuilt binary

Each [GitHub release](https://github.com/ModularDevicesMalleableAudio/pd-toolkit/releases)
publishes prebuilt binaries for users without a Rust toolchain, named
`pdtk-v<version>-<target-triple>`:

- `x86_64-unknown-linux-musl` (fully static)
- `aarch64-unknown-linux-musl` (Raspberry Pi 4; fully static)
- `aarch64-apple-darwin`

The Linux `musl` builds are fully static; the Apple Silicon macOS build links
against the system libraries as usual. Intel Mac users should install with
`cargo install pdtk`. Download the asset matching your platform, put it on your
`PATH`, and run.
This naming scheme is a stable public contract — anything that downloads a
release automatically (e.g. an install script in another repo) can rely on
it; changing `matrix.target` or the asset name in `.github/workflows/release.yml`
is a breaking change for those consumers.

### Build from source

Requires Rust stable (≥ 1.87):

```sh
git clone https://github.com/ModularDevicesMalleableAudio/pd-toolkit
cd pd-toolkit
cargo build --release
# Binary at target/release/pdtk
```

To install into a local `.tools/bin/` directory (matching the `make
install-local` layout):

```sh
cargo install --path . --root .tools
```

### Shell completions

```sh
pdtk completions bash > ~/.local/share/bash-completion/completions/pdtk
pdtk completions zsh  > ~/.zfunc/_pdtk
pdtk completions fish > ~/.config/fish/completions/pdtk.fish
```

---

## Man pages

`build.rs` generates man pages on every `cargo build`, writing them into
Cargo's `OUT_DIR` (so that `cargo publish` accepts the crate).  Use
`make man` to stage them into `./man/` for local viewing or packaging:

```sh
make man                      # stages target/.../out/man/*.1 into ./man/
man -l man/pdtk.1             # top-level man page
man -l man/pdtk-format.1      # per-command page
```

`make install` and `make install-local` invoke `make man` automatically.

---

## Building for Raspberry Pi

```sh
cargo install cross
cross build --release --target aarch64-unknown-linux-musl
# Fully static binary — no glibc required on the Pi
scp target/aarch64-unknown-linux-musl/release/pdtk pi:~/bin/
```

See `Makefile` for `make cross-pi`, `make deploy`, and other targets.

---

## Releasing

Releases are fully automated by `.github/workflows/release.yml` on every push
to `main` (i.e. every merged PR):

1. `.github/workflows/validate.yml` (the same gate `ci.yml` runs on every PR)
   must pass.
2. The version is bumped according to a label on the merged PR — add
   `release:minor` or `release:major` before merging to override the default
   **patch** bump. The bump/tag decision itself lives in
   `.github/scripts/bump-version.sh` (unit-tested by
   `tests/check_bump_version.sh`, run as part of `./tests/run_tests.sh`).
3. Cross-platform binaries are built (see asset list under
   [Prebuilt binary](#prebuilt-binary)) and attached to a new GitHub release,
   and the crate is published to crates.io.

No local publish step is needed or supported — do not run `cargo publish`
by hand.

### Required repository secrets

| Secret | Purpose | Falls back to |
|---|---|---|
| `RELEASE_TOKEN` | Push the version-bump commit/tag to a protected `main` | `GITHUB_TOKEN` (only works if `main` has no required checks blocking `GITHUB_TOKEN` pushes) |
| `CARGO_REGISTRY_TOKEN` | Publish to crates.io | none — the publish step is skipped with a log message if unset |
