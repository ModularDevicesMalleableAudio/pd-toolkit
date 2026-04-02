# pd-else Fixture Attribution

These Pure Data patch files are taken from the **pd-else** library by
Alexandre Torres Porres and contributors.

- **Repository**: https://github.com/porres/pd-else
- **License**: WTFPL (Do What The F\*ck You Want To Public License), Version 2
  — see https://github.com/porres/pd-else/blob/master/License.txt
- **Copyright**: Copyright (C) 2017–2023 Alexandre Torres Porres and others

## Files included

| File | Source directory | Description |
|------|-----------------|-------------|
| `arpeggiator.pd` | Abstractions/Control | MIDI arpeggiator — 5 subpatches, inline width hints, floatatoms, `\$0` send/receive |
| `bpm.pd` | Abstractions/Control | BPM-to-milliseconds converter — simple flat patch with escaped chars |
| `chorus~.pd` | Abstractions/Audio | Audio chorus effect — 2 subpatches, signal chain, inline width hints |
| `clock.pd` | Abstractions/Control | Rhythmic clock generator — 7 subpatches, `\$0`-namespaced sends |
| `compress~.pd` | Abstractions/Audio | Dynamic range compressor — 2 subpatches, route with inline width hint |
| `crusher~.pd` | Abstractions/Extra | Bit crusher — 2 subpatches, signal processing chain |
| `euclid.pd` | Abstractions/Control | Euclidean rhythm generator — floatatom, 2 subpatches |
| `glide.pd` | Abstractions/Control | Portamento / pitch glide — 2 subpatches, multiple inline width hints |
| `gran~.pd` | Abstractions/Extra | Granular synthesis engine — 6 subpatches, complex signal routing |
| `pvoc~.pd` | Abstractions/Extra | Phase vocoder — 5 subpatches, FFT-based spectral processing |
| `tremolo~.pd` | Abstractions/Audio | Tremolo effect — flat signal patch, LFO-controlled amplitude |

## Notes

These files are included as integration test fixtures to exercise the pdtk
parser against real-world abstraction files that use features such as:
- Multi-level subpatch nesting
- `\$0`-prefixed namespaced send/receive names
- Inline `", f N"` width hints
- Backslash-escaped `\$` and `\;` characters
- Deeply nested abstraction graphs
- Signal-rate (`~`) object chains
