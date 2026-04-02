# Subpatch & batch commands

## extract

Extract a subpatch into a standalone abstraction. With `--in-place`, replaces the subpatch block with an abstraction reference.

```bash
pdtk extract file.pd --depth 1 --output my_abs.pd
pdtk extract file.pd --depth 1 --output my_abs.pd --in-place --backup
```

## batch

Run any pdtk command recursively across `.pd` files in a directory.

```bash
pdtk batch src/ validate
pdtk batch src/ format --in-place
pdtk batch src/ find-orphans --continue-on-error
pdtk batch src/ --dry-run validate                # list files without executing
pdtk batch src/ --glob '*.pd' validate --json
```

Exits 1 if any file failed.
