@AGENTS.md

## Claude Code only
- Procedures above are project skills: `xp-slice`, `parallel-slices`, `build-portable` (invoke them by name).
- The code review before merge is `/code-review`.
- A project hook (`.claude/settings.json` → `.claude/hooks/prettier.mjs`) runs Prettier on every `src/` file after each Edit/Write.
- Edit files with Edit/Write, never Bash heredocs/sed/Python.
