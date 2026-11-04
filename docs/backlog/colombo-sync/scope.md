# ColomboSync – one-way folder sync for non-technical users

**Status:** scoped 2026-11-03, backlog reviewed 2026-11-04 (S1 done) · **Implementer:** agent alone · **Tracker:** none (no git remote yet)

## Problem
A person who cannot use the command line needs to keep a copy of a folder from one disk/network drive on another. Robocopy can do it but is unusable for them, and a wrong flag (`/MIR`) silently deletes files on the destination.

## User & job
A non-technical Windows user needs to **copy everything from A to B, safely and repeatably**, so that B is an up-to-date copy of A without ever touching a terminal.

## Success signal
The user creates a sync with the wizard, reads the preview schema, starts it, and later finds it in the history – without help and without losing a file on B they didn't expect to lose.

## Riskiest assumption
**A non-technical user understands what is about to happen** (what gets copied, updated and – in mirror – deleted) from the schema and the preview. The walking skeleton goes through wizard → preview schema → run → history.
Prototype: **approved**, updated version 2026-11-04 – `./prototype.html`. It is the reference for flow, Italian copy and look. Not yet tested with a real non-technical user: **checkpoint 1** after S1b (Copy mode: create a copy, read the preview and say what will happen) and **checkpoint 2** after S2 (mirror only: turn it on, read what gets deleted).
Technical spike on robocopy output: **done** (see Decisions).

## In scope (this round)
- Windows desktop app, as small and light as possible.
- Wizard: pick A (source) → pick B (destination) → name → advanced options → preview → "Start now?".
- A and B: local disks or network drives (mapped letter or UNC `\\server\share`).
- Default mode: copy A → B (robocopy `/E`), never deletes on B.
- Mirror (`/MIR`) as an **advanced option** with a prominent warning; explicit confirmation when the preview shows deletions.
- A schema visible on home and in the wizard/run/result (Storico shows the history legend instead): A → B with counts of new / updated / deleted files, before and after the run.
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
- Stack: **Tauri 2 + Svelte 5** (user, 2026-11-03; WinForms/net48 dropped). Portable release `.exe` (`tauri build --no-bundle`), no installer; WebView2 assumed preinstalled. Size goal: as small as Tauri allows. JSON via `serde_json`; schema as inline SVG.
- Look (user, 2026-11-03, updated 2026-11-04 via the new prototype): **rounded corners, bordered inputs, soft shadows, Nunito bundled in the exe (works offline), light + dark theme with an Auto / Chiaro / Scuro switch**, calm sage/green-teal palette; Copy mode shown as safe (green "Nessun file viene cancellato in B"), Mirror in brick red. Replaces the old look (square corners, no borders, light only).

### Backlog review 2026-11-04 (user-confirmed)
- S1 done; backlog and docs translated to Tauri terms (C# table removed).
- S1b added for the new prototype look; dark theme moved in; Nunito bundled offline (no Google Fonts).
- "Schema always visible" amended: visible on home and wizard/run/result; Storico shows the history legend.
- MirrorGuard simplified, no tree walk: block when A is missing/empty (`read_dir`) or `deleted > new + updated`. Accepted risk: it may block a legitimate big cleanup; revisit if users hit it.
- Path rules (B equal to / inside A) moved from S6 to S2.
- Real run counts: Ok keeps preview counts; Errore/Annullato store one partial total counted from the run's own `/UNILOG`. The preview parser can't be reused: after a copy every file exists in B, so all would count as Updated.
- S5 now blocked by S1 (was S3); S6 blocked by S1, S3, S4 (log viewer); checkpoints 1 and 2 added.
- Challenge: alternatives explorer ran; devil's advocate interrupted by the user, done inline by the planner.

## Open risks
- Readability of the schema and the mirror warning for a non-technical user – checkpoint 2 (after S2); Copy-mode preview – checkpoint 1.
- Very large trees (100k+ files): preview lists are capped at 200 per group, counts stay exact – done in S1.
- Network share disconnecting mid-run – exit ≥ 8 is recorded as Errore in S1; plain-language message and retry in S6.
- MirrorGuard approximation may block a legitimate big cleanup – accepted, see Decisions.
