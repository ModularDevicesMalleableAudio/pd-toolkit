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

List all PD arrays — both classic (`#X array`) and modern (`array define ...`) —
with structured options, schema versioning, and duplicate detection across
files.

```bash
pdtk arrays file.pd
pdtk arrays src/ --json --kind all
pdtk arrays src/ --kind define --templates only
pdtk arrays src/ --schema 1 --json   # legacy v1 envelope
```

Flags:

- `--kind classic|define|all` — filter by array kind. Default is `classic`
  in the first v2 release (preserves row count); pass `--kind all` to
  include `array define` rows.
- `--templates include|exclude|only` — control template-named (`$1`..`$9`)
  arrays.
- `--schema 1|2` — pin output schema. v1 reproduces legacy output
  exactly; v2 (default) adds `schema_version`, `kind`, `is_template`,
  per-row `define`/`classic` payloads, and richer `duplicate_names`.

### v2 JSON shape (excerpt)

```json
{
  "schema_version": 2,
  "arrays": [
    {
      "file": "backend/slew_arrays.pd", "depth": 0, "index": 0,
      "kind": "define", "name": "cc_slew_tau", "size": 16,
      "is_template": false,
      "define": {
        "k": true, "yrange": [0, 128], "pix": null,
        "discarded_tokens": [], "parse_status": "clean"
      }
    },
    {
      "file": "patches/wave.pd", "depth": 1, "index": null,
      "kind": "classic", "name": "waveform_a", "size": 256,
      "is_template": false,
      "classic": {
        "save_flag": 3, "saveit": true,
        "filestyle": "points", "hidename": false
      }
    }
  ],
  "duplicate_names": {}
}
```

The right-anchored parser guarantees `name` and `size` are recovered
correctly even when an unknown future flag appears in `array define`;
unknown / superseded / malformed tokens land in `discarded_tokens`
with a `reason` and the row's `parse_status` becomes `"partial"`.
