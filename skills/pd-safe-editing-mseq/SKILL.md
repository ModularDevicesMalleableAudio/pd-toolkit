---
name: pd-safe-editing-mseq
description: Safe Pure Data (.pd) editing workflow for MSEQ using pdtk only. Use this skill whenever a task touches any .pd file (inspect, search, modify, insert/delete, connect/disconnect, validate, format), even if the requested change seems small.
---

# PD Safe Editing (MSEQ)

## Trigger conditions
- Any request to inspect or modify a `.pd` file
- Any request mentioning object indices, connections, subpatches, or array definitions
- Any request involving `pdtk` commands

## Hard rules
- Use only `.tools/bin/pdtk` for `.pd` read/search/mutation
- Do not use raw text tools on `.pd` (`read`, `edit`, `write`, `grep`, `sed`, `awk`, scripts)
- Do not hand-edit `#X connect` lines
- Validate after every mutation: `.tools/bin/pdtk validate <file>`

## Default workflow
1. `pdtk list <file>` to identify objects and depth
2. `pdtk search <file|dir>` / `pdtk connections` / `pdtk trace` as needed
3. Apply edits with `pdtk modify|insert|delete|connect|disconnect`
4. Run `pdtk validate <file>`
5. Optional final pass: `pdtk format <file> --in-place` then validate again

## Mutation strategy
- Content change only: `pdtk modify`
- Structural change: `pdtk insert` / `pdtk delete` (never manual renumbering)
- Wiring change: `pdtk connect` / `pdtk disconnect`
- Prefer smallest safe change-set with explicit validation

## Narrow exception
- Direct text edits are allowed only for `#X array` and `#A` data blocks
- Confirm target lines are non-indexed with `pdtk list`
- Run `pdtk validate` immediately after

## References
- `references/workflow.md` — command templates and common recipes
- `references/checklists.md` — pre-flight and post-flight checks
- `/home/kris/code/pd-toolkit/skills/edit-and-lint-pd/SKILL.md` — full pdtk reference
