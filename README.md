# pdtk — Pure Data Patch Toolkit

A safe, fast command-line tool for parsing, inspecting, editing, validating,
and auto-formatting Pure Data (`.pd`) patch files — without breaking
connections.

Every mutating command validates the result before writing. Mutations that
would produce an invalid patch are refused, and the source file is never
modified unless explicitly requested with `--in-place`.

---

## Install

### Download a pre-built binary

```sh
# Linux x86_64
curl -fsSL https://github.com/<user>/pd-toolkit/releases/latest/download/pdtk-x86_64-linux \
    -o pdtk && chmod +x pdtk

# Linux aarch64 (Raspberry Pi 4, static musl — no runtime deps)
curl -fsSL https://github.com/<user>/pd-toolkit/releases/latest/download/pdtk-aarch64-linux-musl \
    -o pdtk && chmod +x pdtk

# macOS Apple Silicon
curl -fsSL https://github.com/<user>/pd-toolkit/releases/latest/download/pdtk-aarch64-macos \
    -o pdtk && chmod +x pdtk
```

### Build from source

Requires Rust stable (≥ 1.77):

```sh
git clone https://github.com/<user>/pd-toolkit
cd pd-toolkit
cargo build --release
# Binary at target/release/pdtk
```

### Install into a project

```sh
# For a sequencer / live-coding project:
./scripts/install-pdtk.sh          # auto-detects platform, downloads latest release
./scripts/install-pdtk.sh v0.3.0   # pin to a specific version
```

### Shell completions

```sh
pdtk completions bash > ~/.local/share/bash-completion/completions/pdtk
pdtk completions zsh  > ~/.zfunc/_pdtk
pdtk completions fish > ~/.config/fish/completions/pdtk.fish
```

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
pdtk deps     patch.pd --missing               # abstractions not found on disk
pdtk trace    patch.pd --from 0                 # what does object 0 connect to?
pdtk diff     old.pd new.pd --ignore-coords    # what changed (ignoring layout)?

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
pdtk batch    src/ format --in-place          # auto-format all files
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

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Validation / lint errors |
| 2 | Parse / usage errors |
| 3 | I/O errors |

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

## Man pages

Man pages are generated at build time in the `man/` directory:

```sh
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
