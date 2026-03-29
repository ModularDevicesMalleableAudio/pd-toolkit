# Layout command

## format

Reposition objects using topological layering. **Only X/Y coordinates change — object order, content, and connections are byte-identical.**

```bash
pdtk format file.pd --in-place
pdtk format file.pd --depth 0 --in-place     # top-level only
pdtk format file.pd --dry-run                 # preview without writing
pdtk format file.pd --grid 20 --hpad 10 --margin 20 --in-place
```

Always validate after formatting:

```bash
pdtk format file.pd --in-place && pdtk validate file.pd
```
