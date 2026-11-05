# ColomboSync – agent guide

Desktop app (Windows, macOS, Linux) that lets a non-technical user copy folder A → B safely: wizard, mandatory preview, A→B schema in the side panel (home, wizard, run, result; Storico shows the history legend instead), history. Engine: robocopy on Windows, rsync on macOS (openrsync) and Linux (GNU rsync). UI in Italian.

**Stack:** Tauri 2 (Rust, `src-tauri/`) + Svelte 5 + Vite + TypeScript (`src/`). No installer. Deliverables: Windows portable exe (`npm run portable`), macOS `.app`/`.dmg` (ad-hoc signed, not notarized: first launch right-click → Apri), Linux AppImage.

## Read first
- `docs/backlog/colombo-sync/backlog.md` – slices S1…S7, acceptance criteria, shared conventions.
- `docs/backlog/colombo-sync/scope.md` – what's in/out, decisions (log new decisions and new Italian copy here).
- `docs/backlog/colombo-sync/prototype.html` – flow and **exact Italian copy**. Look: design canvas https://claude.ai/artifact/Re562wrVhSASbEL78RWg1D ("Eclissi" logo, dark Apple-style teal theme + light variant, system font; tokens in `src/theme.css`).
- `docs/agents/glossary.md` – domain names to use in code.
- `docs/agents/robocopy.md` (Windows) and `docs/agents/rsync.md` (macOS/Linux) – engine facts: exit codes, flags, output formats, never parse labels.
- `docs/agents/github.md` – repo, CI matrix, `gh` token recipe (the active `gh` account is NOT the repo owner).
- `docs/agents/xp.md` – how we work (TDD, CI, review, definition of done).

## Commands
```
npm test                              # Svelte tests (vitest, jsdom)
npm run check                         # svelte-check / TypeScript
cd src-tauri && cargo fmt && cargo test
cd src-tauri && cargo clippy --all-targets -- -D warnings
npm run tauri dev                     # run the app in dev
npm run portable                      # Windows exe: src-tauri\target\release\colombo-sync.exe
npx tauri build --bundles app,dmg     # macOS only (CI builds it)
npx tauri build --bundles appimage    # Linux only (CI builds it)
```
First release build takes ~5 min. Unix-only code can't be compiled here (Windows machine): push a `slice/**` branch and read CI – see `docs/agents/github.md` for the commands.

## Skills
- `xp-slice` – implement one backlog slice test-first.
- `parallel-slices` – run several slices in git worktrees and merge them.
- `build-portable` – build the app for this OS and/or fetch the CI artifacts for the others.

## Architecture rules
- **Rust owns all logic, I/O and every user-facing message** (`Explanation {title, advice}`; history stores `Cause`s, not prose): engine args, running it, preview parsing, path rules, `syncs.json`, `history.jsonl`, logs in the data dir (`%LOCALAPPDATA%\ColomboSync`, `~/Library/Application Support/ColomboSync`, `$XDG_DATA_HOME`/`~/.local/share/ColomboSync`). `src-tauri/src/` is one directory per vertical slice, like `src/` on the Svelte side: `syncs/`, `wizard/`, `run/`, `history/`, `engine/` and `shared/` (what 2+ slices use: `storage`, `error_messages`, `path_rules`, `elevation`, `blocking`). Each slice keeps its `#[tauri::command]`s in its own `commands.rs`, registered in `lib.rs`.
- **Engine boundary = `src-tauri/src/engine/mod.rs`**: `cfg(windows)` re-exports `engine/robocopy.rs`, `cfg(unix)` re-exports `engine/rsync.rs`. Runner, history, MirrorGuard, UI never learn which engine runs. Other platform code is `cfg`-gated too.
- **Svelte owns presentation**: one folder per vertical slice under `src/` (`syncs/`, `wizard/` with one component per step, `run/`, `history/`, `schema/` for the `SchemaView.svelte` side panel), plus `shared/` for what two or more slices use. Each slice owns its components, tests, `api.ts` (the `invoke`/event wrappers) and `types.ts`. Theme tokens in `src/theme.css`. Talks to Rust only via `invoke` and events.
- Domain logic stays free of Tauri types where possible, so it is unit-testable with plain `cargo test`.
- No elevation, no installer, nothing from the backlog's "Later" section.

## Conventions
- Test first; commit at every green (`feat(sN): …`). Full suite green before handing back.
- Every slice branch gets `/code-review` before merge; it found real data-loss bugs every time (path rules incl. A inside B, links/aliases, normalization, MirrorGuard trusting the UI, overwriting unreadable files, non-atomic writes). Treat folder-safety code as hostile-input code.
- `cargo fmt` + clippy clean; English identifiers, Italian UI strings copied from the prototype. New Italian copy not in the prototype: list it in the slice report and in scope.md "Decisions during implementation".
- On this machine shell heredocs/Python halve backslashes in source text: edit files with the Edit/Write tools. In TS tests use `String.raw` for Windows paths.
- Minimum code for the criteria; no traits/abstractions with a single implementation.
