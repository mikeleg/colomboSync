---
name: parallel-slices
description: Orchestrate several ColomboSync backlog slices in parallel with plain git worktrees and subagents, then merge them one by one with the full suite green. Use when asked to parallelize the backlog, run slices in parallel, or merge slice branches.
---

# Parallel slices

The orchestrator (main thread) plans, delegates and verifies. Subagents write code.

## 1. Pick the wave
From the dependency graph at the top of `docs/backlog/colombo-sync/backlog.md`: every slice whose "Blocked by" is merged on `main`. S1 always alone first.

## 2. One worktree per slice (never Claude's `isolation: "worktree"`)
```
git worktree add ../colomboSync-s3 -b slice/s3 main
cd ../colomboSync-s3 && npm ci
```

## 3. Brief each subagent
Use the `Agent` tool, all in one message. Model: `sonnet` for M slices, `haiku` only for S slices that are pure logic. Prompt must include:
- worktree absolute path – work and commit only there;
- "Use the `xp-slice` skill for slice `Sx`" (path `.claude/skills/xp-slice/SKILL.md`);
- the slice's "Do not touch" and the hot spots (`lib.rs` handler list, `App.svelte`, wizard steps) – minimal additive edits;
- the report format of `xp-slice`.

## 4. Verify, don't trust
For each finished branch, in its worktree: run `cargo test`, `npm test`, `npm run check`; read the diff (`git diff main...slice/sx`); run `/code-review` on it.

## 5. Merge one at a time
Merge first the branch others depend on (S3 before S5).
```
git checkout main && git merge --no-ff slice/s3
cd src-tauri && cargo test && cd .. && npm test && npm run check
```
Resolve conflicts in hot spots by keeping both sides (both commands registered, both steps present). Suite red after merge → fix on main before the next merge.

## 6. Clean up
```
git worktree remove ../colomboSync-s3 && git branch -d slice/s3
```
Then start the next wave (e.g. S5 after S3).
