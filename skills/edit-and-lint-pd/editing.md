# Editing commands

All mutating commands default to stdout. Use `--in-place` to write back, `--backup` to keep a `.bak`.

## insert

Insert object at index, auto-renumber connections.

```bash
pdtk insert file.pd --depth 0 --index 3 --entry '#X obj 50 75 delay 500;' --in-place
```

## delete

Remove object, delete its connections, renumber remaining.

```bash
pdtk delete file.pd --depth 0 --index 5 --in-place
```

## modify

Change object class/args in place — index, coordinates, and connections unchanged.

```bash
pdtk modify file.pd --depth 0 --index 3 --text "route 1 2 3" --in-place
```

## connect

Add a patch cord.

```bash
pdtk connect file.pd --depth 0 --src 0 --outlet 0 --dst 2 --inlet 0 --in-place
```

## disconnect

Remove a specific patch cord.

```bash
pdtk disconnect file.pd --depth 0 --src 0 --outlet 0 --dst 2 --inlet 0 --in-place
```

## renumber

Manually shift connection indices ≥ `--from` by `--delta`.

```bash
pdtk renumber file.pd --depth 0 --from 2 --delta 1 --in-place
```

## rename-send

Rename send/receive names across files (handles s/r, s~/r~, throw~/catch~, GUI fields).

```bash
pdtk rename-send file.pd --from clock_main --to clock_renamed --in-place
pdtk rename-send src/ --from audio_bus --to audio_main --dry-run
```

## Common pattern: add object and wire it in

```bash
pdtk insert file.pd --depth 0 --index 5 --entry '#X obj 200 150 + 1;' --in-place
pdtk connect file.pd --depth 0 --src 4 --outlet 0 --dst 5 --inlet 0 --in-place
pdtk connect file.pd --depth 0 --src 5 --outlet 0 --dst 6 --inlet 0 --in-place
pdtk validate file.pd
```
