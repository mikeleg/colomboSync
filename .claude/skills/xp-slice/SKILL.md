---
name: xp-slice
description: Implement one ColomboSync backlog slice (S1, S1b, S2…S6 or later slices) test-first, XP style – red/green/refactor, commit at every green, full suite and portable build before handing back. Use when asked to implement, build, do or continue a slice/story from docs/backlog/colombo-sync/backlog.md.
---

# Implement a slice, XP style

Input: a slice id (e.g. `S3`) and, if parallel, the worktree path you work in.

## 1. Load context (read, don't skim)
- `CLAUDE.md` (repo root) – commands and boundaries.
- The slice section in `docs/backlog/colombo-sync/backlog.md` + its "Shared conventions" section.
- `docs/agents/glossary.md`, `docs/agents/robocopy.md` (if the slice touches the engine), `docs/agents/xp.md`.
- `docs/backlog/colombo-sync/prototype.html` only for the screens of this slice: copy Italian texts and CSS tokens verbatim.
- The existing code you will change.

## 2. Test list
Write down (in your head or a scratch note) one test per acceptance criterion plus every test the slice names. Order: pure Rust logic first, then Tauri commands, then Svelte components.

## 3. Loop, per test
1. **Red** – write the test, run only it, confirm it fails for the expected reason.
2. **Green** – smallest code that passes. No speculative options.
3. **Refactor** – on green only; domain names, no duplication you introduced.
4. **Commit** – `git add -A && git commit -m "feat(sN): <criterion>"` (or `test`/`refactor`). End message with the repo's Co-Authored-By line if one is configured.

Commands:
```
cd src-tauri && cargo test <name>      # Rust
npm test -- <file>                     # Svelte (vitest)
```

## 4. Integrate
Before reporting:
```
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
npm test && npm run check
npm run portable
```
All must pass. Then run the slice's main flow by hand on the exe if you can, otherwise say "manual flow not verified".

## 5. Report
- Criteria → test that covers each (or "manual only").
- Files touched, commits made.
- Anything ambiguous you stopped on, anything not done.

## Boundaries
- Only this slice. Respect its "Do not touch". Nothing from "Later".
- Shared hot spots (merge conflicts): `src-tauri/src/lib.rs` command list, `src/App.svelte`, wizard step order. Keep edits there minimal and additive.
- If an acceptance criterion contradicts the prototype or another slice, stop and report.
