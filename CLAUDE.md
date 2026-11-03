# ColomboSync – agent guide

Windows desktop app that lets a non-technical user copy folder A → B with robocopy, safely: wizard, mandatory preview, always-visible A→B schema, history. UI in Italian.

**Stack:** Tauri 2 (Rust, `src-tauri/`) + Svelte 5 + Vite + TypeScript (`src/`). Deliverable: one portable exe, no installer.

## Read first
- `docs/backlog/colombo-sync/backlog.md` – slices S1…S6, acceptance criteria, shared conventions (incl. C#→Tauri table).
- `docs/backlog/colombo-sync/scope.md` – what's in/out, decisions.
- `docs/backlog/colombo-sync/prototype.html` – approved look, flow and **exact Italian copy**.
- `docs/agents/glossary.md` – domain names to use in code.
- `docs/agents/robocopy.md` – engine facts (exit codes, flags, UTF-16 log, never parse labels).
- `docs/agents/xp.md` – how we work (TDD, CI, definition of done).

## Commands
```
npm test                              # Svelte tests (vitest, jsdom)
npm run check                         # svelte-check / TypeScript
cd src-tauri && cargo test            # Rust unit + integration tests
cd src-tauri && cargo clippy --all-targets -- -D warnings
npm run tauri dev                     # run the app in dev
npm run portable                      # release exe: src-tauri\target\release\colombo-sync.exe
```
First release build takes ~5 min.

## Skills
- `xp-slice` – implement one backlog slice test-first.
- `parallel-slices` – run several slices in git worktrees and merge them.
- `build-portable` – build and smoke-test the exe.

## Architecture rules
- **Rust owns all logic and I/O**: robocopy args, running it, preview parsing, path rules, error messages, `syncs.json`, `history.jsonl`, logs in `%LOCALAPPDATA%\ColomboSync\`. One module per concern in `src-tauri/src/`, exposed through `#[tauri::command]`s registered in `lib.rs`.
- **Svelte owns presentation**: one component per wizard step in `src/lib/`, `SchemaView.svelte` always visible, theme tokens in `src/theme.css`. Talks to Rust only via `invoke` and events.
- Domain logic stays free of Tauri types where possible, so it is unit-testable with plain `cargo test`.
- No elevation, no installer, nothing from the backlog's "Later" section.

## Conventions
- Test first; commit at every green (`feat(sN): …`). Full suite green before handing back.
- `cargo fmt` + clippy clean; English identifiers, Italian UI strings copied from the prototype.
- Minimum code for the criteria; no traits/abstractions with a single implementation.
