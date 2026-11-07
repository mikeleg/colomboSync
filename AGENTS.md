# ColomboSync – agent guide

Guide for any coding agent (Codex, Cursor, Gemini, Copilot, Claude Code, …) and for humans. Everything needed to work on the project is in this repo; nothing lives on external pages.

Desktop app (Windows/macOS/Linux): a non-technical user copies folder A → B safely. Wizard, mandatory preview, A→B schema side panel (Storico shows a summary panel instead: Riepilogo, Per sincronizzazione, legend), history. Engine: robocopy (Windows), rsync (macOS openrsync, Linux GNU rsync). UI in Italian.

**Stack:** Tauri 2 (Rust, `src-tauri/`) + Svelte 5 + Vite + TS (`src/`). No installer. Deliverables: Windows portable exe, macOS `.app`/`.dmg` (ad-hoc signed, not notarized: first launch right-click → Apri), Linux AppImage.

## Read first
- `docs/backlog/colombo-sync/backlog.md` – slices, acceptance criteria, shared conventions.
- `docs/backlog/colombo-sync/scope.md` – in/out, decisions; log new decisions and new Italian copy here.
- `docs/backlog/colombo-sync/prototype.html` – flow and **exact Italian copy**.
- `docs/design/` – the look ("Eclissi": logo and reference screens, dark teal + light variant, system font). Tokens in `src/theme.css`, app icon in `src-tauri/icons/app-icon.svg`.
- `docs/agents/glossary.md` – domain names for code.
- `docs/agents/robocopy.md`, `docs/agents/rsync.md` – exit codes, flags, output formats; never parse labels.
- `docs/agents/github.md` – repo, CI matrix, `gh` token recipe (active `gh` account is NOT the repo owner).
- `docs/agents/xp.md` – TDD, CI, review, definition of done.

## Procedures
Step-by-step checklists in `.claude/skills/*/SKILL.md`. They are plain markdown: any agent can follow them (ignore the front matter).
- `xp-slice` – implement one backlog slice test-first, up to CI and review.
- `parallel-slices` – several slices in parallel with git worktrees, then merge one at a time.
- `build-portable` – build the Windows exe, fetch the macOS/Linux bundles from CI.

## Commands
```
npm test / npm run check / npm run format:check   # vitest, svelte-check, Prettier: fixes what it can, then fails on what is left (CI only checks)
cd src-tauri && cargo fmt && cargo test && cargo clippy --all-targets -- -D warnings
npm run tauri dev                             # dev
npm run portable                              # src-tauri\target\release\colombo-sync.exe (~5 min first time)
npx tauri build --bundles app,dmg | appimage  # macOS / Linux only (CI builds them)
```
The dev machine is Windows: Unix-only code can't compile here. Push a `slice/**` branch and read CI (`docs/agents/github.md`).

## Architecture
- **Rust owns all logic, I/O and every user-facing message** (`Explanation {title, advice}`; history stores `Cause`s, not prose): engine args and run, preview parsing, path rules, `syncs.json`, `history.jsonl`, logs in the data dir (`%LOCALAPPDATA%\ColomboSync`, `~/Library/Application Support/ColomboSync`, `$XDG_DATA_HOME` or `~/.local/share/ColomboSync`).
- `src-tauri/src/`: one dir per vertical slice – `syncs/`, `wizard/`, `run/`, `history/`, `engine/`, `shared/` (used by 2+ slices: `domain` – `SyncMode`, `SavedSync`, `Run`, `Outcome`, `Cause` –, `storage`, `sync_store` (`syncs.json`), `error_messages`, `path_rules`, `mirror_guard`, `elevation`, `blocking`). Depend downward only on `engine/` and `shared/` (exceptions: `run` reads `PreviewMemo` from `wizard/preview.rs`; `syncs` reads `history`). Each slice's `#[tauri::command]`s live in its `commands.rs`, registered in `lib.rs` (`shared/elevation.rs` is the `is_elevated` command).
- **Engine boundary = `engine/mod.rs`**: `cfg(windows)` → `robocopy.rs`, `cfg(unix)` → `rsync.rs`. Nothing else knows which engine runs; other platform code is `cfg`-gated too.
- **Svelte owns presentation**: `src/` mirrors the slices (`syncs/`, `wizard/` one component per step, `run/`, `history/`, `schema/` = `SchemaView.svelte`, `shared/`). Each slice owns components, tests, `api.ts` (`invoke`/event wrappers), `types.ts`. Talks to Rust only via `invoke` and events.
- Domain logic free of Tauri types where possible (plain `cargo test`).
- No elevation, no installer, nothing from the backlog's "Later".

## Conventions
- Test first; commit at every green (`feat(sN): …`); full suite green before handing back.
- `main` changes only through a pull request, owner included (GitHub ruleset, no bypass): work on a branch, never push `main`.
- Before every push or PR run locally every check CI runs: `npm run format:check`, `npm test`, `npm run check`, `cargo fmt --check`, clippy, `cargo test`. Line endings are LF everywhere (`.gitattributes`), so the local Prettier check matches the Windows runner.
- Every slice branch gets a code review for correctness bugs before merge – it found real data-loss bugs every time (path rules incl. A inside B, links/aliases, normalization, MirrorGuard trusting the UI, overwriting unreadable files, non-atomic writes). Treat folder-safety code as hostile-input code.
- fmt + clippy + Prettier clean (Prettier formats `src/` and the HTML/TS/JSON files; `*.md` is ignored). English identifiers, Italian UI strings from the prototype; new copy not in the prototype goes in the slice report and scope.md "Decisions during implementation".
- Look: colors only as tokens from `src/theme.css`, no inline colors. Mirror/deletion stays red and loud; Specchio (the mode) is violet (`--mirror`).
- Svelte 5 best practices: `$derived` (not `$effect`) for computed state; `$effect` never forwards data to a parent or writes derivable state; every `{#each}` keyed; `svelte-ignore` only with a comment saying why.
- Shell heredocs, `sed` and Python halve backslashes here: write files with your file-editing tool. TS tests: `String.raw` for Windows paths.
- Minimum code for the criteria; no traits/abstractions with a single implementation.
- Untracked local files `demo.html`, `src/demo.ts` (browser demo with Tauri mocks) and `HANDOFF.md` are never committed.
