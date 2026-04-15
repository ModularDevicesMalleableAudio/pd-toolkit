# Checklists

## Pre-flight
- Identify file and target depth (`pdtk list`)
- Identify exact object indices before editing
- Confirm whether change is content-only (`modify`) vs structural (`insert/delete`)
- For structural edits, plan affected wiring changes explicitly

## Post-flight
- Run `.tools/bin/pdtk validate <file>`
- If layout changed, run `.tools/bin/pdtk format <file> --in-place` and validate again
- Re-run `pdtk list`/`pdtk connections` for touched indices
- Confirm escaped dollars are preserved in object args (`\$1`, `\$2`, ...)

## Forbidden recovery patterns
- Do not "quick-fix" by manually editing connection lines
- Do not run ad-hoc sed/awk/perl substitutions on `.pd`
- If `pdtk` cannot express the change, stop and report limitation
