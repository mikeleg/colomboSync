---
name: parallel-slices
description: Orchestrate several ColomboSync backlog slices in parallel with plain git worktrees and subagents, review each branch, then merge them one at a time with the full suite and CI green on all three OS. Use when asked to parallelize the backlog, run slices in parallel, or merge slice branches.
---

# Parallel slices

The orchestrator (main thread) plans, delegates and verifies. Subagents write code.

## 1. Pick the wave
From the dependency graph at the top of `docs/backlog/colombo-sync/backlog.md`: every slice whose "Blocked by" is merged on `main`. Slices that touch the same files (see hot spots) are **not** parallel – sequence them.

Conflict hot spots: `src/App.svelte`, `src/wizard/Wizard.svelte`, `StepPreview.svelte`, `src-tauri/src/lib.rs` (handler list and managed state), the slices' `commands.rs`, `run/runner.rs`, `engine/mod.rs`.

## 2. One worktree per slice (never Claude's `isolation: "worktree"`)
```
git worktree add ../colomboSync-s8 -b slice/s8 main
cd ../colomboSync-s8 && npm ci
```

## 3. Brief each subagent
Use the `Agent` tool, all in one message. Model: `sonnet` for M slices, `haiku` only for S slices that are pure logic. Give each the **parallel context**, since it knows nothing else:
- worktree absolute path – work and commit only there; branch `slice/sx`;
- "Use the `xp-slice` skill for slice `Sx`" (path `.claude/skills/xp-slice/SKILL.md`) and to read `CLAUDE.md` and `docs/agents/github.md`;
- which other slices run at the same time, which files they will touch, and that edits to the hot spots must be minimal and additive (one line in the handler list, one state registration, one tab/step);
- the slice's "Do not touch";
- to push the branch with the owner-token recipe (`docs/agents/github.md`) so CI runs on Windows, macOS and Linux, and to report the run id and result per OS; never to push `main`;
- the report format of `xp-slice` (incl. the list of new Italian copy).

## 4. Verify, don't trust
For each finished branch, in its worktree: `cargo test`, `npm test`, `npm run check`; CI green on all three OS (`gh run watch <id> -R mikeleg/colomboSync --exit-status`, with `export GH_TOKEN=$(gh auth token -u mikeleg)`); read the diff (`git diff main...slice/sx`); run **`/code-review` on every branch before merge**. It has found real data-loss bugs in every slice so far (path rules, links/aliases, normalization, MirrorGuard trusting the UI, overwriting unreadable files, non-atomic writes). Send findings back to the subagent (or fix them) with a test each; re-review until clean.

## 5. Merge one at a time
`main` accepts changes only through a pull request (GitHub ruleset, no bypass, owner included): never `git push origin main`. Merge first the branch others depend on, one PR at a time:
```
gh pr create -R mikeleg/colomboSync --base main --head slice/s8 --fill
gh pr merge <n> -R mikeleg/colomboSync --merge --delete-branch     # only with CI green on the PR
git checkout main && git pull && cd src-tauri && cargo test && cd .. && npm test && npm run check
```
Merge `main` into the next slice branch, push it again and wait for CI before merging its PR. Resolve hot-spot conflicts by keeping both sides (both commands registered, both states managed, both steps/tabs present). Suite red after a merge → fix it with a new branch and PR before the next merge. Check CI on `main` after each merge.

## 6. Clean up and record
```
git worktree remove ../colomboSync-s8 && git branch -d slice/s8
git push origin --delete slice/s8        # with the token recipe's credential helper
```
Record each slice's new Italian copy in `scope.md` "Decisions during implementation" and mark the slice done in `backlog.md`. Then start the next wave.
