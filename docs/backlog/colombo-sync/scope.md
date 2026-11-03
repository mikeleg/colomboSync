# ColomboSync – one-way folder sync for non-technical users

**Status:** scoped 2026-11-03 · **Implementer:** agent alone · **Tracker:** none (no git remote yet)

## Problem
A person who cannot use the command line needs to keep a copy of a folder from one disk/network drive on another. Robocopy can do it but is unusable for them, and a wrong flag (`/MIR`) silently deletes files on the destination.

## User & job
A non-technical Windows user needs to **copy everything from A to B, safely and repeatably**, so that B is an up-to-date copy of A without ever touching a terminal.

## Success signal
The user creates a sync with the wizard, reads the preview schema, starts it, and later finds it in the history – without help and without losing a file on B they didn't expect to lose.

## Riskiest assumption
**A non-technical user understands what is about to happen** (what gets copied, updated and – in mirror – deleted) from the schema and the preview. The walking skeleton goes through wizard → preview schema → run → history.
Prototype: **done and approved** (2026-11-03) – `./prototype.html` (also https://claude.ai/artifact/NkW69fhkzXfCbZ7F1eTVRS). It is the reference for flow, Italian copy and look. Not yet tested with a real non-technical user.
Technical spike on robocopy output: **done** (see Decisions).

## In scope (this round)
- Windows desktop app, as small and light as possible.
- Wizard: pick A (source) → pick B (destination) → name → advanced options → preview → "Start now?".
- A and B: local disks or network drives (mapped letter or UNC `\\server\share`).
- Default mode: copy A → B (robocopy `/E`), never deletes on B.
- Mirror (`/MIR`) as an **advanced option** with a prominent warning; explicit confirmation when the preview shows deletions.
- A schema always visible: A → B with counts of new / updated / deleted files, before and after the run.
- Mandatory preview (`/L`) before every run.
- Saved syncs, re-runnable from the list.
- Sync history: date, sync name, mode, counts, outcome, duration, link to the log.
- UI in Italian.

## Out of scope / non-goals
- macOS / Linux / rsync – user chose robocopy on Windows.
- SSH / remote destinations – only disks and network shares.
- Scheduling (nightly runs, Task Scheduler) – manual start only in this MVP.
- Two-way sync, multiple sources to one destination – A → B only.
- Exclusion filters (file types, folders) – adds a whole wizard step; later.
- Notifications, tray icon, auto-update, installer – single exe is enough.
- Multilanguage – Italian only.

## Constraints
- **Stack: Tauri 2 + Svelte 5 (TypeScript)** – single portable `.exe`, no installer; WebView2 (preinstalled on Windows 10/11). Schema drawn as inline SVG.
- Engine: `robocopy.exe` from `%SystemRoot%\System32`, launched as a child process.
- Storage: one JSON file for saved syncs, one JSON-lines file for history, one robocopy log per run – all in `%LOCALAPPDATA%\ColomboSync\`.
- App must **not** require elevation (mapped drives disappear under an elevated token).

## Decisions taken during scoping
- Robocopy instead of rsync; A/B are disks or network drives; wizard ends with preview + "start?"; history required; smallest possible app (user, 2026-11-03).
- Robocopy facts from spike (2026-11-03):
  - Exit codes 0–7 = success (bitmask 1 copied, 2 extras, 4 mismatch), ≥ 8 = failure. A preview returning 3 is **not** an error.
  - Always pass `/R:1 /W:1` – the default retry (`/R:1000000 /W:30`) hangs for days on one locked file.
  - Stdout is OEM codepage (Italian dates come out garbled) and class labels (`New File`, `*EXTRA File`) may be localized → **never parse labels**. Use `/L /NJH /NJS /NDL /NC /BYTES /FP /NP /UNILOG:<tmp>` and classify by path: path under B → will be deleted; path under A whose relative path exists in B → updated; otherwise → new.
- ~~Stack WinForms/net48 over Tauri/Electron~~ → **Tauri 2 + Svelte 5** (user veto, 2026-11-03). Portable: the release `.exe` (`tauri build --no-bundle`), no installer; WebView2 assumed preinstalled (Windows 10/11). Size goal relaxed from < 1 MB to "as small as Tauri allows". JSON via `serde_json`; schema as inline SVG.
- Look (user, 2026-11-03, via prototype): Material-style **tonal surfaces, no borders, square corners**, calm sage/green-teal palette; Copy mode shown as safe (green "Nessun file viene cancellato in B"), Mirror in brick red. Light theme only in MVP. Square corners keep WinForms painting simple.
- JSON via `serde_json` (Rust side); the app stays a single exe with no DLLs.

## Open risks
- Readability of the schema and the mirror warning for a non-technical user – carried into S1/S2 (or answered by the prototype).
- Very large trees (100k+ files): preview list must stay responsive – carried into S1 (stream the log, show counts, not every line).
- Network share disconnecting mid-run – carried into S1 (exit ≥ 8 → history row "failed" with log).
