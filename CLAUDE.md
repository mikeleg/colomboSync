# ColomboSync – agent guide

Desktop app (Windows/macOS/Linux): a non-technical user copies folder A → B safely. Wizard, mandatory preview, A→B schema side panel (Storico shows a summary panel instead: Riepilogo, Per sincronizzazione, legend), history. Engine: robocopy (Windows), rsync (macOS openrsync, Linux GNU rsync). UI in Italian.

**Stack:** Tauri 2 (Rust, `src-tauri/`) + Svelte 5 + Vite + TS (`src/`). No installer. Deliverables: Windows portable exe, macOS `.app`/`.dmg` (ad-hoc signed, not notarized: first launch right-click → Apri), Linux AppImage.

## Read first
- `docs/backlog/colombo-sync/backlog.md` – slices, acceptance criteria, shared conventions.
- `docs/backlog/colombo-sync/scope.md` – in/out, decisions; log new decisions and new Italian copy here.
- `docs/backlog/colombo-sync/prototype.html` – flow and **exact Italian copy**. Look: https://claude.ai/artifact/Re562wrVhSASbEL78RWg1D ("Eclissi", dark teal + light variant, system font; tokens in `src/theme.css`).
- `docs/agents/glossary.md` – domain names for code.
- `docs/agents/robocopy.md`, `docs/agents/rsync.md` – exit codes, flags, output formats; never parse labels.
- `docs/agents/github.md` – repo, CI matrix, `gh` token recipe (active `gh` account is NOT the repo owner).
- `docs/agents/xp.md` – TDD, CI, review, definition of done.

## Commands
```
npm test / npm run check / npm run format     # vitest, svelte-check, Prettier (CI runs format:check)
cd src-tauri && cargo fmt && cargo test && cargo clippy --all-targets -- -D warnings
npm run tauri dev                             # dev
npm run portable                              # src-tauri\target\release\colombo-sync.exe (~5 min first time)
npx tauri build --bundles app,dmg | appimage  # macOS / Linux only (CI builds them)
```
Unix-only code can't compile here: push a `slice/**` branch and read CI (`docs/agents/github.md`).

Skills: `xp-slice` (one slice test-first), `parallel-slices` (worktrees + merge), `build-portable`.

## Architecture
- **Rust owns all logic, I/O and every user-facing message** (`Explanation {title, advice}`; history stores `Cause`s, not prose): engine args and run, preview parsing, path rules, `syncs.json`, `history.jsonl`, logs in the data dir (`%LOCALAPPDATA%\ColomboSync`, `~/Library/Application Support/ColomboSync`, `$XDG_DATA_HOME` or `~/.local/share/ColomboSync`).
- `src-tauri/src/`: one dir per vertical slice – `syncs/`, `wizard/`, `run/`, `history/`, `engine/`, `shared/` (used by 2+ slices: `domain` – `SyncMode`, `SavedSync`, `Run`, `Outcome`, `Cause` –, `storage`, `sync_store` (`syncs.json`), `error_messages`, `path_rules`, `mirror_guard`, `elevation`, `blocking`). Depend downward only on `engine/` and `shared/` (exceptions: `run` reads `PreviewMemo` from `wizard/preview.rs`; `syncs` reads `history`). Each slice's `#[tauri::command]`s live in its `commands.rs`, registered in `lib.rs` (`shared/elevation.rs` is the `is_elevated` command).
- **Engine boundary = `engine/mod.rs`**: `cfg(windows)` → `robocopy.rs`, `cfg(unix)` → `rsync.rs`. Nothing else knows which engine runs; other platform code is `cfg`-gated too.
- **Svelte owns presentation**: `src/` mirrors the slices (`syncs/`, `wizard/` one component per step, `run/`, `history/`, `schema/` = `SchemaView.svelte`, `shared/`). Each slice owns components, tests, `api.ts` (`invoke`/event wrappers), `types.ts`. Talks to Rust only via `invoke` and events.
- Domain logic free of Tauri types where possible (plain `cargo test`).
- No elevation, no installer, nothing from the backlog's "Later".

## Conventions
- Test first; commit at every green (`feat(sN): …`); full suite green before handing back.
- Every slice branch gets `/code-review` before merge – it found real data-loss bugs every time (path rules incl. A inside B, links/aliases, normalization, MirrorGuard trusting the UI, overwriting unreadable files, non-atomic writes). Treat folder-safety code as hostile-input code.
- fmt + clippy + Prettier clean. English identifiers, Italian UI strings from the prototype; new copy not in the prototype goes in the slice report and scope.md "Decisions during implementation".
- Svelte 5 best practices: `$derived` (not `$effect`) for computed state; `$effect` never forwards data to a parent or writes derivable state; every `{#each}` keyed; `svelte-ignore` only with a comment saying why.
- Shell heredocs/Python halve backslashes here: edit with Edit/Write. TS tests: `String.raw` for Windows paths.
- Minimum code for the criteria; no traits/abstractions with a single implementation.
