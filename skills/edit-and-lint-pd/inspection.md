# Inspection commands

## parse

Summary statistics: object count, connections, max depth, warnings.

```bash
pdtk parse file.pd
pdtk parse file.pd --json
```

## list

All objects with `[depth:index]` addresses, class, and coordinates.

```bash
pdtk list file.pd
pdtk list file.pd --depth 1
pdtk list file.pd --json
```

## validate

Check connection indices are in range, canvas pairs balanced.

```bash
pdtk validate file.pd
pdtk validate file.pd --strict   # also warn on duplicate connections
```

Exits 0 on success, 1 on errors, 2 on parse failures.

## lint

Validate + layout style checks (overlap detection).

```bash
pdtk lint file.pd
pdtk lint file.pd --json
```

## stats

Complexity metrics: class histogram, fan-in/out, orphan count.

```bash
pdtk stats file.pd
pdtk stats src/             # aggregate across directory
```

## connections

All patch cords to/from a specific object.

```bash
pdtk connections file.pd --index 3
pdtk connections file.pd --index 3 --depth 1
```

## arrays

List all PD arrays; detect duplicates across files.

```bash
pdtk arrays file.pd
pdtk arrays src/ --json
```
