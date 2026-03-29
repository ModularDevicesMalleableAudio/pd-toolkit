# Search & analysis commands

## search

Find objects by class and/or text pattern (glob or regex).

```bash
pdtk search file.pd --type route
pdtk search src/ --type send --text "clock_*"
pdtk search file.pd --regex --text "trig_\\d+"
```

## find-orphans

Objects with zero connections. Can auto-delete.

```bash
pdtk find-orphans file.pd
pdtk find-orphans file.pd --delete --in-place --backup
```

## find-displays

Connected debug displays (floatatom, nbx, etc). Can auto-delete.

```bash
pdtk find-displays file.pd
pdtk find-displays file.pd --delete --in-place
```

## trace

Follow signal/message path downstream from an object via BFS.

```bash
pdtk trace file.pd --from 0
pdtk trace file.pd --from 0 --to 5          # shortest path
pdtk trace file.pd --from 0 --max-hops 3
```

## diff

Structural diff between two patches (objects added/removed/modified, connections changed).

```bash
pdtk diff old.pd new.pd
pdtk diff old.pd new.pd --ignore-coords     # essential when comparing before/after format
```

## deps

List abstraction dependencies. `--missing` shows only unresolved ones.

```bash
pdtk deps file.pd
pdtk deps file.pd --missing
pdtk deps src/ --recursive --json
```
