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
- Desktop app for Windows, macOS and Linux, as small and light as possible.
- Wizard: pick A (source) → pick B (destination) → name → advanced options → preview → "Start now?".
- A and B: local disks or network drives (Windows: mapped letter or UNC `\\server\share`; macOS/Linux: mounted folders, e.g. `/Volumes/...`).
- Default mode: copy A → B (robocopy `/E` / rsync `-rt`), never deletes on B.
- Mirror (robocopy `/MIR` / rsync `--delete`) as an **advanced option** with a prominent warning; explicit confirmation when the preview shows deletions.
- A schema visible on home and in the wizard/run/result (Storico shows the history legend instead): A → B with counts of new / updated / deleted files, before and after the run.
- Mandatory preview (`/L`) before every run.
- Saved syncs, re-runnable from the list.
- Sync history: date, sync name, mode, counts, outcome, duration, link to the log.
- UI in Italian.

## Out of scope / non-goals
- SSH / remote destinations – only disks and network shares.
- Scheduling (nightly runs, Task Scheduler) – manual start only in this MVP.
- Two-way sync, multiple sources to one destination – A → B only.
- Exclusion filters (file types, folders) – adds a whole wizard step; later.
- Notifications, tray icon, auto-update, installer – single exe is enough.
- Multilanguage – Italian only.

## Constraints
- **Stack: Tauri 2 + Svelte 5 (TypeScript)**, no installer. Deliverables: Windows portable `.exe` (WebView2, preinstalled on Windows 10/11); macOS `.app`/`.dmg`, ad-hoc signed, not notarized (first launch: right-click → Apri); Linux AppImage. macOS/Linux bundles are built by GitHub Actions. Schema drawn as inline SVG.
- Engine, launched as a child process and picked by `cfg(target_os)`: `robocopy.exe` from `%SystemRoot%\System32` on Windows; `rsync` from `PATH` on macOS (openrsync) and Linux (GNU rsync).
- Storage: one JSON file for saved syncs, one JSON-lines file for history, one engine log per run – in `%LOCALAPPDATA%\ColomboSync\` (Windows), `~/Library/Application Support/ColomboSync` (macOS), `$XDG_DATA_HOME/ColomboSync` or `~/.local/share/ColomboSync` (Linux).
- App must **not** require elevation on any OS (on Windows mapped drives disappear under an elevated token).

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

## Decisions during implementation (2026-11-04)
Copy and behaviour not in the prototype, decided while implementing; review with a real user at the checkpoints.
- S1b: new copy "Non riesco ad aprire questa cartella. Il disco o la rete sono collegati?"; picker header "Cartella scelta: …"; all drives shown as "Disco X:" (prototype distinguishes local/USB/rete – a mapped network letter also shows as a disk); "Dettagli tecnici" box not ported.
- S2: also rejects A inside B ("La cartella da copiare non può stare dentro la destinazione."); paths normalized lexically (., .., \\?\); MirrorGuard has two reasons: "A sembra vuota o scollegata…" and "Con lo specchio verrebbero cancellati da B più file di quelli che arrivano da A. Controlla di aver scelto le cartelle giuste."; a mirror run without a matching preview is refused ("Prima controlla le due cartelle con l’anteprima, poi avvia."); mirror counts come from Rust's last preview, not the UI; green "Non cancello mai niente in B" callout hidden when mirror is on. Accepted risk: the count rule may block a deliberate large cleanup.
- S3: a damaged syncs.json is set aside as syncs.json.corrotto-<timestamp> and the home starts empty with "Le sincronizzazioni salvate non erano leggibili: ne ho messo da parte una copia."; save failure says "Non riesco a salvare la sincronizzazione: …"; last outcome matched by sync id; sync name in log file names capped at 60 chars.
- S4: "Non riesco a leggere lo storico.", "Sto leggendo lo storico…"; log viewer shows only the last 2 MB with "… (mostro solo l’ultima parte del registro)"; it only opens files inside logs\.
- S5: "Nessun file è stato copiato.", "Il file già copiato resta in B."; partial count excludes the file in flight; the schema shows only the animated arrow during a run (no bar/growing counters as in the prototype); ETA in seconds only.
- S6: preview failures "Non riesco a controllare le cartelle: …" / "Non riesco a controllare le cartelle." + "Controlla che A e B siano raggiungibili e riprova." with "Riprova", no log link at preview; "Non trovo la cartella B (…). La cartella di rete è raggiungibile?"; "Non ho il permesso di usare la cartella B (…). Controlla i permessi e riprova."; generic "Non completato: la copia si è fermata per un problema. Apri il registro per vedere i dettagli." and card note "L'ultima copia non è stata completata. Nel registro trovi i dettagli."; history stores the cause (code + folder), messages derived on read; after "Riprova" the sync behaves as saved.
- Not verified at runtime yet: offline Nunito, window close during a run, elevated-token banner, real network failures (53/67), manual flow on the exe. One vitest flake seen once (1 in ~8 runs) after the S2 merge, not reproduced since.

## Decisions 2026-11-04 (afternoon)
- Scope change (user): macOS and Linux builds too. Engine there is **rsync** (Windows keeps robocopy). macOS app is **not signed** with an Apple Developer ID (ad-hoc signature only; first launch needs right-click → Apri). Linux ships as AppImage.
- Repo on GitHub: `mikeleg/colomboSync` (private). GitHub Actions build and test Windows, macOS and Linux.
- Look (user): "Eclissi" logo in teal (origami colours), dark Apple-style screens – design canvas (copied into `docs/design/` on 2026-11-07) replaces prototype.html for look only; prototype.html stays the source for flow and Italian copy. Nunito dropped for the system font.

## Decisions 2026-11-05 (CI runners)
- Probed on the runners: `macos-14` ships samba rsync 2.6.9, not openrsync; `macos-15` (15.8.2), `macos-26` and `macos-latest` (= macos-26) ship openrsync (protocol 29); `ubuntu-latest` is Ubuntu 26.04 with rsync 3.4.1, `ubuntu-24.04` has 3.2.7. `macos-latest` now gets runners, but the label is unreliable (it never did on 2026-11-04), so macOS is pinned to `macos-15`.
- CI and the release build run on `macos-15` (tests against openrsync, what the engine targets); macOS 14 and older (samba rsync) are not tested. Linux: tests on `ubuntu-latest`; the release AppImage is built on `ubuntu-22.04` (glibc baseline).
- Moving the tests to `macos-15` exposed a real engine difference: openrsync on macOS 15 prints a deleted folder as `*deleting old-dir` (no `/`), so a mirror preview counted the folder as a deleted file (MirrorGuard counts and the file list were off by the number of deleted folders). The parser now asks B whether a deleted name is a folder (`engine/rsync.rs`, test `a_deleted_folder_without_a_trailing_slash_is_not_a_file`).
- No macOS runner has Rosetta (`arch -x86_64` fails), so the Intel half of the universal dmg is never run in CI.

## Decisions 2026-11-05 (README, license, release notes)
- Added `README.md` (Italian, user-first: promise, screenshots from `docs/screenshots/`, benefits, download and install, FAQ, beta status; a short English "Per sviluppatori" section at the end) and an MIT `LICENSE` (Copyright 2026 Michele Gatti).
- The repo stays **private for now** (owner): the README links to `https://github.com/mikeleg/colomboSync/releases/latest` and uses relative links, so they work for whoever has access. The shields.io release badge will not render while the repo is private.
- Release notes are user-facing: install notes first, the generated `feat`/`fix` commit list at the end inside a collapsed `<details>` block ("Dettagli tecnici (modifiche)"; `release.yml`). The body of the existing beta draft is replaced by `docs/release-notes/v0.1.0-beta.1.md`.
- New Italian copy (not in the prototype): `README.md` (all of it) and `docs/release-notes/v0.1.0-beta.1.md`; the install notes are copied unchanged from `.github/release-notes-it.md`. The screenshots `anteprima.png`, `home.png`, `mirror.png`, `copia.png`, `storico.png`, `chiaro.png` in `docs/screenshots/` are referenced but added separately.
- Corrected the "Da aggiornare" group copy (`StepPreview.svelte`, mirrored in `prototype.html`). The engine (robocopy/rsync) replaces any file of B that differs from A, including when B's copy is newer (A always wins; `/XO` not used), so "più vecchi" was wrong.
  - Old: "File che in B sono più vecchi di quelli in A."
  - New: "File che ci sono sia in A sia in B ma sono diversi: verranno sostituiti con quelli di A."
  - Why: the group lists every differing file, not only older ones; the sentence must not promise that a newer copy in B is kept.

## Decisions 2026-11-05 (improve-ui)
- "Nuova cartella" (issue #2) is offered only in the **Dove** picker (B), not in Da dove (A) (owner). It shows while browsing inside a folder, not in the list of drives; it asks a name, creates the folder in the one being browsed with `std::fs::create_dir` (never the parents, never reuses an existing one) and chooses it. B's checks (`check_destination`) still run on Avanti as before; creating needs none.
- The name is validated in Rust, with the Windows rules on every OS (the folder may be read by Windows later): trimmed; not empty; at most 255 bytes; none of `< > : " / \ | ? *` nor control characters (so never a path, `.` or `..`); no trailing dot; not a reserved device name (CON, PRN, AUX, NUL, CONIN$, CONOUT$, COM1-9, LPT1-9, also COM¹²³/LPT¹²³), with or without an extension. The parent must be an absolute path.
- New Italian copy (not in the prototype). UI: button "Nuova cartella"; input label "Nome della nuova cartella"; placeholder "Per esempio: Backup"; buttons "Crea" and "Annulla". Messages (title / advice, shown together next to the input):
  - "Scrivi un nome per la nuova cartella."
  - "Il nome è troppo lungo."
  - "Il nome non può contenere questi caratteri: \ / : * ? " < > |" / "Scrivi solo il nome della cartella, senza percorso."
  - "Questo nome non si può usare." / "Scegline un altro: niente punto in fondo, né nomi come CON o NUL."
  - "Esiste già una cartella con questo nome." / "Scegli un altro nome." (also shown when a file has that name)
  - "Non trovo la cartella in cui creare la nuova." / "Il disco è collegato?"
  - "Non ho il permesso di creare una cartella qui." / "Scegli un altro posto o controlla i permessi."
  - "Il disco è in sola lettura: non posso crearci cartelle." / "Scegli un altro disco."
  - "Non riesco a creare la cartella." / "Controlla che il disco o la rete siano collegati e riprova." (any other error)
- **Storico side panel (issue #4)** (owner). It is no longer "Come leggere lo storico" (component `HistoryPanel.svelte`; the aside is labelled "Riepilogo dello storico"). Three separate sections, top to bottom:
  - "Riepilogo": tiles "Copie fatte", "Riuscite", "Con errore", "Annullate", "File copiati", "File cancellati in B". Computed in Rust from `history.jsonl` by the new command `list_runs` (summary built by `history/summary.rs`), no new stored data. "File copiati" = new + updated of every run, or the `partial` count of failed/stopped runs (which keep no new/updated); "File cancellati in B" = sum of `deleted` (failed/stopped runs store 0).
  - "Per sincronizzazione": one entry per sync found in the history (matched by sync id; runs from before ids belong to none, but count in the totals), with its current name from `syncs.json` (the name in its latest run if it is gone). Shows "Ultima copia riuscita: <data>" or "Nessuna copia riuscita finora."; when its latest run ended in Errore, a warning made in Rust: "L’ultima copia è finita con un errore il <data>." Warnings first, then by name. A stopped run is not a warning. A sync that was never run does not appear.
  - "Cosa vogliono dire le etichette": the four chips with their one-line explanations, same texts as before, smaller and set apart at the bottom.
  - The old single "Ultima copia riuscita: <when>." line is gone (now per sync). Empty history: "Non hai ancora fatto nessuna copia. Quando ne farai una, qui vedrai i numeri e lo stato di ogni sincronizzazione." instead of zeros (the legend stays). If the summary cannot be read, the rows still show and the panel keeps only the legend.
  - `prototype.html` still has the old panel (heading "Come leggere lo storico", two counters, "Ultima copia riuscita"); it was not redone, this entry is the reference.
- **Specchio has its own colour, distinct from Errore** (issue #4, owner): Specchio is a mode the user chooses, not a problem. New tokens `--mirror` / `--on-mirror` in `theme.css`: light `#6b3fd0` / `#ffffff` (about 6.4:1), dark `#a78bfa` / `#1a0b3d` (about 6.7:1). `.chip.mirror` uses them everywhere the mode label shows: Storico rows, home cards, side panel, the pill in the A → B schema, "Specchio attivo" in the wizard. Red stays for what is about deletion or failure: the "Da cancellare in B" group, "Eliminati da B", the red B node and arrow, red "Cancellati N", warnings, and the Errore chip.
- **Storico filters (issue #5)** (owner). Four native selects above the list, combinable (a run must match all), filtering in Svelte over the runs already loaded by `list_runs` (no Rust change). They show only when the history has runs. State lives in `History.svelte`: kept while the Registro modal opens and closes, not stored across restarts. The side-panel numbers (#4) stay about the whole history.
  - Labels and options: "Sincronizzazione" (Tutte + one entry per sync, by name, A–Z; runs match by sync id, old lines without one by name; a renamed sync shows under its latest name), "Esito" (Tutti, OK, Errore, Annullato), "Modalità" (Tutte, Copia, Specchio), "Periodo" (Sempre, Ultimi 7 giorni, Ultimi 30 giorni: counted back from now, by the run's start time).
  - With any filter on: "N copie su M" ("1 copia su M" for one) and the link-button "Togli i filtri", which resets all four. When nothing matches: "Nessuna copia con questi filtri." instead of the list (the "Togli i filtri" link stays above it).
  - `types.ts` `Run` gets the optional `syncId` that Rust already sends.
- **Registro (issue #1)** (owner). The log window (opened from Storico and from the result screen) now has a summary above the raw log and highlights the engine's errors.
  - Summary (section labelled "Riepilogo" for screen readers): the mode and outcome chips and the figures of the Storico rows ("Nuovi", "Aggiornati" or the partial count, "Cancellati", "Durata"; same labels, same `RunStats.svelte`). For an Errore run also the explanation, title and advice, in Italian: `list_runs` now sends each run with the `explanation` Rust makes from the stored cause (`RunReport`, as `run_sync` already did); no new stored data. The summary shows even when the log file is gone ("Il registro di questa copia non è più disponibile.").
  - Errors in the raw log: `read_run_log` returns `{ text, errorLines }` (line numbers from 0 of the text shown). Rust decides, per engine: robocopy = a line with a Windows error code (`ERROR 5 (0x00000005)`, any code, not a file line); rsync = a line written under rsync's own name (`rsync: …`, `rsync error: …`, `rsync warning: …` on GNU, `rsync(<pid>): error|warning: …` on openrsync; read from the GNU and openrsync sources, confirmed only by the CI integration test). Labels are never read. The line after a robocopy error (Windows' own sentence) is not highlighted.
  - Error lines are shown bold in the "danger" colours of the theme (`--on-danger-soft` on `--danger-soft`, both themes). New copy: the button "Vai al primo errore" (shown only when the log has an error line; scrolls to the first one and moves the focus there).
  - Errors in the part of a long log that is not shown (the log is cut to its last 2 MB): Rust scans that part too (in chunks, never loading the whole file) and puts the error lines it finds, at most 200, at the top of the text, above the truncation notice, under one line of new copy: "Errori nella parte non mostrata del registro:". They count as error lines like the others ("Vai al primo errore" reaches them).

## Decisions 2026-11-07 (docs self-contained)
- Everything needed lives in the repo (owner): the "Eclissi" design canvas, until now only on claude.ai, is copied as static HTML into `docs/design/` (logo, home, anteprima specchio, sto copiando) with a README; no doc links to external pages.
- `AGENTS.md` is the agent guide for every tool; `CLAUDE.md` imports it (`@AGENTS.md`) and adds only what is specific to Claude Code (skills, `/code-review`, Prettier hook).
- `main` only through pull requests, owner included (owner): GitHub ruleset "main: solo via pull request" on the default branch – PR required with 0 approvals (nobody else reviews), no force-push, no deletion, no bypass actors. Required status checks not set (GitHub Actions is blocked by billing on the owner account; a required check would block every merge). The release bump commit goes through a PR too; tags are not affected.
- The browser demo (`demo.html` + `src/demo.ts`, backend mocked with `mockIPC`, used for screenshots and visual checks) is now in the repo (owner): type-checked by `npm run check` and formatted by Prettier; its mocks follow the commands' API.
