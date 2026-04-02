# AI Agent Setup

pdtk ships a skill file that tells AI coding agents how to use the tool for
all `.pd` file operations. The skill covers the full command surface, the
critical depth convention, the object-indexing rules, and safe mutation
workflows.

The skill lives in `skills/edit-and-lint-pd/` in this repository. Once
installed it is automatically loaded by compatible agents when working on `.pd`
files, or you can invoke it explicitly with `/skill:edit-and-lint-pd`.

---

## 1 — pi (and compatible harnesses)

[pi](https://github.com/badlogic/pi) discovers skills from `.pi/skills/` in
the project directory, `~/.pi/agent/skills/` globally, and `skills/`
directories of installed packages.

### Project-level installation

```sh
# From inside your project
cp -r path/to/pd-toolkit/skills/edit-and-lint-pd .pi/skills/
```

The agent will load it automatically whenever you work on `.pd` files.

### Global installation

```sh
mkdir -p ~/.pi/agent/skills
cp -r path/to/pd-toolkit/skills/edit-and-lint-pd ~/.pi/agent/skills/
```

Available in every project on the machine.

### If pd-toolkit is a sibling repo

Add to your project's `.pi/settings.json`:

```json
{
  "skills": ["../pd-toolkit/skills"]
}
```

pi will discover the skill directly from the pd-toolkit source tree without
copying anything.

### Invoke explicitly

```
/skill:edit-and-lint-pd
```

---

## 2 — Claude Code (Anthropic `claude` CLI)

Claude Code reads `CLAUDE.md` files from the project root and parent
directories, and also loads skills from `~/.claude/skills/`.

### Option A — project CLAUDE.md (recommended for single projects)

Add to your project's `CLAUDE.md` (create it if it doesn't exist):

```markdown
## Pure Data (.pd) editing

All .pd file work must use pdtk. Read the skill before touching any .pd file:
```

Then paste the full content of `skills/edit-and-lint-pd/SKILL.md` into
`CLAUDE.md`, followed by the reference sections you need.

### Option B — global skills directory

```sh
mkdir -p ~/.claude/skills
cp -r path/to/pd-toolkit/skills/edit-and-lint-pd ~/.claude/skills/
```

pi can also load these skills via `.pi/settings.json`:

```json
{
  "skills": ["~/.claude/skills"]
}
```

### Option C — AGENTS.md (for Claude and other agents)

If your project already has `AGENTS.md`, add a section pointing at the skill:

```markdown
## Pure Data Editing

All .pd file work must use pdtk. Before touching any .pd file, load and follow
the skill at `.pi/skills/edit-and-lint-pd/SKILL.md`.

Tool binary: `.tools/bin/pdtk` (or wherever installed — must be on PATH).
Depth convention: `--depth 0` = top-level, `--depth 1` = first subpatch level.
```

---

## 3 — OpenAI Codex / GitHub Copilot Workspace

These agents read `AGENTS.md` (OpenAI Codex), `.github/copilot-instructions.md`
(Copilot Workspace), or similar project instruction files.

### AGENTS.md

Add to your project's `AGENTS.md`:

```markdown
## Pure Data (.pd) editing policy

All .pd edits must go through pdtk. Never hand-edit connection indices.

Tool: `pdtk` (must be on PATH, or at `.tools/bin/pdtk`)
Depth: `--depth 0` = top-level canvas, `--depth 1` = first subpatch level

### Workflow
1. `pdtk list file.pd` — inspect objects and indices
2. `pdtk validate file.pd` — check structure before editing
3. Use pdtk commands for all mutations (insert/delete/modify/connect/…)
4. `pdtk validate file.pd` — verify after every change
5. `pdtk format file.pd --in-place` — clean up layout (optional)

### Critical rules
- Use `pdtk insert` / `pdtk delete` — NEVER hand-edit #X connect lines
- All mutating commands default to stdout; use `--in-place` to write back
- Always run `pdtk validate` after structural changes
```

Then append the contents of `skills/edit-and-lint-pd/SKILL.md` (and the
reference .md files) for full command coverage.

### .github/copilot-instructions.md

Same content as the `AGENTS.md` section above.

---

## 4 — Cursor / Windsurf / similar IDE agents

These read `.cursorrules` (Cursor) or equivalent project-level instruction
files.

Add to `.cursorrules` or `.windsurfrules`:

```
For Pure Data (.pd) files: always use pdtk for all operations.
Never hand-edit #X connect lines after structural changes.
Run `pdtk validate file.pd` after every mutation.
Full command reference: skills/edit-and-lint-pd/SKILL.md in this repo.
```

For full coverage, append the skill content directly.

---

## 5 — Claude.ai / web interfaces (Projects)

For Claude Projects or any web interface that supports custom instructions:

1. Open **Project Settings → Custom Instructions**
2. Paste the full content of `skills/edit-and-lint-pd/SKILL.md`
3. Optionally append the reference files (`editing.md`, `inspection.md`, etc.)

The combined text is under 8 KB and fits comfortably in most context windows.

For ad-hoc conversations without a project, upload `SKILL.md` as a file
attachment at the start of the conversation.

---

## 6 — Any LLM (generic system prompt)

Paste the following into your system prompt, adjusting the pdtk path as needed:

```
You have access to pdtk, a CLI for safely editing Pure Data (.pd) patch files.
Binary: pdtk (must be on PATH)

DEPTH CONVENTION: --depth 0 = top-level canvas, --depth 1 = first subpatch level.

WORKFLOW:
1. pdtk list file.pd       — inspect objects and indices
2. pdtk validate file.pd   — check before editing
3. pdtk <command> ...      — make changes
4. pdtk validate file.pd   — verify after

CRITICAL: always use pdtk insert/delete, never hand-edit #X connect lines.

OBJECT INDEX RULES:
- Objects are 0-based, counted by definition order
- #X obj, #X msg, #X text, #X floatatom, #X symbolatom, #X restore = objects
- #N canvas, #X connect, #X coords, #X declare, #X f N, #A = NOT objects
- #X restore closes a subpatch and is indexed in the PARENT canvas

KEY COMMANDS:
  pdtk parse file.pd                                  # stats
  pdtk list file.pd [--depth N] [--json]              # list objects
  pdtk validate file.pd                               # check connections
  pdtk insert file.pd --depth N --index I --entry '...' --in-place
  pdtk delete file.pd --depth N --index I --in-place
  pdtk modify file.pd --depth N --index I --text '...' --in-place
  pdtk connect file.pd --depth N --src I --outlet O --dst J --inlet K --in-place
  pdtk disconnect file.pd --depth N --src I --outlet O --dst J --inlet K --in-place
  pdtk format file.pd --in-place                      # layout (coords only)
  pdtk rename-send file.pd --from name --to name --in-place
  pdtk find-orphans file.pd [--delete --in-place]
  pdtk find-displays file.pd [--delete --in-place]
  pdtk trace file.pd --from I [--to J]
  pdtk diff old.pd new.pd [--ignore-coords]
  pdtk deps file.pd [--missing]
  pdtk extract file.pd --depth N --output abs.pd [--in-place]
  pdtk batch dir/ <command> [--continue-on-error]
```

---

## Keeping the skill up to date

When you upgrade pdtk, also update the installed skill:

```sh
# If installed as a copy
cp -r path/to/pd-toolkit/skills/edit-and-lint-pd .pi/skills/

# If using the sibling-repo settings.json approach — nothing to do,
# the agent always reads from the current pd-toolkit source
```

The skill is versioned alongside the pdtk binary in the pd-toolkit repository.
