# Workflow Recipes

## Inspect and locate
```bash
.tools/bin/pdtk list path/to/file.pd
.tools/bin/pdtk search path/to/file.pd --text "*pattern*"
.tools/bin/pdtk search path/to/file.pd --type route
.tools/bin/pdtk connections path/to/file.pd --index 12 --depth 0
```

## Safe edits
```bash
.tools/bin/pdtk modify path/to/file.pd --depth 0 --index 12 --text "route 1 2 3" --in-place
.tools/bin/pdtk insert path/to/file.pd --depth 0 --index 20 --entry '#X obj 200 140 t b f;' --in-place
.tools/bin/pdtk delete path/to/file.pd --depth 0 --index 20 --in-place
.tools/bin/pdtk connect path/to/file.pd --depth 0 --src 10 --outlet 0 --dst 11 --inlet 0 --in-place
.tools/bin/pdtk disconnect path/to/file.pd --depth 0 --src 10 --outlet 0 --dst 11 --inlet 0 --in-place
```

## Validate and format
```bash
.tools/bin/pdtk validate path/to/file.pd
.tools/bin/pdtk format path/to/file.pd --in-place
.tools/bin/pdtk validate path/to/file.pd
```

## Search arrays across repo
```bash
.tools/bin/pdtk arrays .
.tools/bin/pdtk search . --text "*array define my_array*"
```
