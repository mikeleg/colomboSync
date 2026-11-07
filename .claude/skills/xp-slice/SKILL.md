---
name: xp-slice
description: Implement one ColomboSync backlog slice (S1…S7 or later slices) test-first, XP style – red/green/refactor, commit at every green, full suite, CI on all three OS and a /code-review before handing back. Use when asked to implement, build, do or continue a slice/story from docs/backlog/colombo-sync/backlog.md.
---

# Implement a slice, XP style

Input: a slice id (e.g. `S8`) and, if parallel, the worktree path you work in.

## 1. Load context (read, don't skim)
- `CLAUDE.md` (repo root) – commands and boundaries.
- The slice section in `docs/backlog/colombo-sync/backlog.md` + its "Shared conventions" section; `scope.md` for decisions already taken.
- `docs/agents/glossary.md`, `docs/agents/xp.md`; `docs/agents/robocopy.md` / `docs/agents/rsync.md` if the slice touches the engine; `docs/agents/github.md` if it has Unix-only code.
- `docs/backlog/colombo-sync/prototype.html` for flow and **exact Italian copy** of this slice's screens; `docs/design/` and `src/theme.css` for look. Tokens only, no inline colors.
- The existing code you will change.

## 2. Test list
One test per acceptance criterion plus every test the slice names. Order: pure Rust logic first, then Tauri commands, then Svelte components.

## 3. Loop, per test
1. **Red** – write the test, run only it, confirm it fails for the expected reason.
2. **Green** – smallest code that passes. No speculative options.
3. **Refactor** – on green only; domain names, no duplication you introduced.
4. **Commit** – `git add -A && git commit -m "feat(sN): <criterion>"` (or `test`/`refactor`/`fix`). End the message with the Co-Authored-By line the session gives you.

Commands:
```
cd src-tauri && cargo test <name>      # Rust
npm test -- <file>                     # Svelte (vitest)
```

Rules while coding:
- **Edit files with the Edit/Write tools.** On this machine shell heredocs and Python halve backslashes in source text. In TS tests write Windows paths with `String.raw`.
- **Messages belong to Rust.** A user-facing failure is an `Explanation {title, advice}` made in Rust from a `Cause {code, folder}` and the `Moment` (preview/run). History stores the `Cause`, never prose; the UI never builds sentences. Map by engine exit code or error code, never by English/localized text.
- **Folder-safety code is hostile-input code.** Reviews found data-loss bugs in every slice: A inside B (both directions), links/aliases/symlinks, `..`/case/Unicode normalization, MirrorGuard trusting what the UI sent (use Rust's `PreviewMemo`), overwriting a file that can't be read, writing `syncs.json`/`history.jsonl` non-atomically (write temp + rename). Write a test for each before you rely on it.
- **Platform code is `cfg`-gated** and goes through `engine/mod.rs`; shared code never names robocopy or rsync. You can't compile Unix code on Windows: keep it small, then use CI (step 5).
- **New Italian copy** not in the prototype: keep a list for the report.

## 4. Integrate (local, Windows)
```
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
npm test && npm run check
npm run portable
```
All must pass. Run the slice's main flow by hand on the exe if you can, otherwise say "manual flow not verified".

## 5. CI and review
- Push the slice branch with the owner-token recipe from `docs/agents/github.md` and watch the run: Windows, macOS and Linux must all be green (fmt, clippy, tests). Fix on the branch until they are. Never push to `main`.
- Run `/code-review` on the branch diff (`git diff main...HEAD`), fix every real finding with a test, re-run steps 4–5. Do not hand back before this.

## 6. Report
- Criteria → test that covers each (or "manual only").
- Files touched, commits made, CI run id and result per OS, `/code-review` findings fixed.
- **New Italian copy** not in the prototype (exact strings) – the orchestrator records it in `scope.md` "Decisions during implementation".
- Anything ambiguous you stopped on, anything not done, "not opened by hand" for macOS/Linux bundles.

## Boundaries
- Only this slice. Respect its "Do not touch". Nothing from "Later".
- Shared hot spots (merge conflicts): `src/App.svelte`, `src/wizard/Wizard.svelte`, `StepPreview.svelte`, the `lib.rs` handler list and managed state, the slices' `commands.rs`, `run/runner.rs`, `engine/mod.rs`. Keep edits there minimal and additive.
- If an acceptance criterion contradicts the prototype or another slice, stop and report.
