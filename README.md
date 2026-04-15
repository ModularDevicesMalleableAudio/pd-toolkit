# pdtk — Pure Data Patch Toolkit

A safe, fast command-line tool for parsing, inspecting, editing, validating,
and auto-formatting Pure Data (`.pd`) patch files — without breaking
connections.

Every mutating command validates the result before writing. Mutations that
would produce an invalid patch are refused, and the source file is never
modified unless explicitly requested with `--in-place`.

---

## Quick start

```sh
# Inspect a patch
pdtk parse    patch.pd           # object count, depth, warnings
pdtk list     patch.pd           # [depth:index] class args for every object
pdtk validate patch.pd           # check all connection indices are in range
pdtk stats    patch.pd           # fan-in, fan-out, class histogram

# Find things
pdtk search   patch.pd --type route             # find all route objects
pdtk search   patch.pd --type send --text "clk*"  # sends whose name starts with clk
pdtk find-orphans  patch.pd                     # objects with no connections
pdtk find-displays patch.pd                     # connected debug number boxes
pdtk deps     patch.pd --missing                # abstractions not found on disk
pdtk trace    patch.pd --from 0                 # what does object 0 connect to?
pdtk diff     old.pd new.pd --ignore-coords     # what changed (ignoring layout)?

# Edit safely
pdtk insert   patch.pd --depth 0 --index 2 --entry '#X obj 50 75 bang;' --in-place
pdtk delete   patch.pd --depth 0 --index 5 --in-place
pdtk modify   patch.pd --depth 0 --index 3 --text 'route 1 2 3' --in-place
pdtk connect  patch.pd --depth 0 --src 0 --outlet 0 --dst 2 --inlet 0 --in-place
pdtk rename-send patch.pd --from clock_main --to clock_renamed --in-place

# Layout
pdtk format   patch.pd --in-place              # auto-reposition objects
pdtk lint     patch.pd                         # validate + detect overlaps

# Subpatch operations
pdtk extract  patch.pd --depth 1 --output my_abs.pd --in-place

# Batch
pdtk batch    src/ validate                    # validate all .pd files in src/
pdtk batch    src/ format --in-place           # auto-format all files
```

---

## Commands

| Category | Command | Description |
|---|---|---|
| **Inspection** | `parse` | Object count, connection count, depth, warnings |
| | `list` | List every indexed object with address and class |
| | `validate` | Check connection index ranges and canvas balance |
| | `lint` | Validate + detect bounding-box overlaps |
| | `stats` | Per-file metrics: fan-in/out, class histogram, orphans |
| | `connections` | List all patch cords to/from one object |
| | `arrays` | List all PD arrays with name and size |
| **Search** | `search` | Find objects by class name or text pattern (glob/regex) |
| | `find-orphans` | Objects with zero connections |
| | `find-displays` | Connected debug display widgets |
| | `trace` | BFS forward trace or path-find between two objects |
| | `diff` | Structural diff (objects added/removed/modified, cords) |
| | `deps` | Abstraction dependency list, optionally filtered to missing |
| **Editing** | `insert` | Insert object, renumber connections automatically |
| | `delete` | Delete object and its cords, renumber remaining |
| | `modify` | Change class/args in place, preserving index and connections |
| | `connect` | Add a patch cord (refuses duplicates and out-of-range) |
| | `disconnect` | Remove a specific patch cord |
| | `renumber` | Manually shift connection indices by a delta |
| | `rename-send` | Rename s/r pairs atomically, including GUI fields |
| **Layout** | `format` | Auto-reposition objects (connections byte-identical) |
| **Subpatch** | `extract` | Extract subpatch into standalone abstraction |
| **Utilities** | `batch` | Apply any command recursively across a directory |
| | `completions` | Generate shell tab-completion scripts |

---

## Safety guarantees

- **`format`** only modifies X/Y coordinate fields — an internal assertion
  refuses to write if any connection line was altered.
- **All mutating commands** run `validate` on the result before writing. A
  failed validation prevents any file modification.
- **`--in-place`** is never the default. Stdout is the default output for
  mutation commands.
- **`--backup`** writes `<file>.bak` before overwriting.

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

### Check a project before deploying to hardware

Validate every patch and list any abstractions that can't be found on disk:

```sh
pdtk batch src/ validate
pdtk deps  src/main.pd --missing
```

Both commands exit non-zero if problems are found, so they drop cleanly into a
`Makefile` or CI step.

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
| 1 | Validation / lint errors |
| 2 | Parse / usage errors |
| 3 | I/O errors |

---

## Install

### Build from source

Requires Rust stable (≥ 1.77):

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

Man pages are generated into `man/` by `build.rs` when you run `cargo build`.
They are not committed to the repository.

```sh
cargo build                   # generates man/ if it doesn't exist yet
man -l man/pdtk.1             # top-level man page
man -l man/pdtk-format.1      # per-command page
```

---

## Building for Raspberry Pi

```sh
cargo install cross
cross build --release --target aarch64-unknown-linux-musl
# Fully static binary — no glibc required on the Pi
scp target/aarch64-unknown-linux-musl/release/pdtk pi:~/bin/
```

See `Makefile` for `make cross-pi`, `make deploy`, and other targets.
