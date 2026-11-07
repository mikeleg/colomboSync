# Backlog – ColomboSync

Scope: ./scope.md · Prototype (flow, copy, look): ./prototype.html · Implementer: agent alone · Tracker: none (markdown only)

Order is dependency order. Checkpoints are not slices: they are a real non-technical user trying the portable exe, before the next slice starts.

```
S1 (done) ── S1b (new look + wizard details) ──┬── [checkpoint 1: Copy mode] ── S2 (mirror) ── [checkpoint 2: mirror]
     │                                         └── S3 (saved syncs) ──┐
     ├── S4 (history) ───────────────────────────────────────────────┴── S6 (plain-language errors) ──► S7 (macOS + Linux with rsync) ──► S8 (release pipeline)
     └── S5 (progress + stop)
```

- S3, S4 and S5 are independent of S2. S4 and S5 need only S1 (S4 reads `history.jsonl`, S5 extends the run).
- **S2, S3 and S6 touch the same Svelte files** (`Wizard`, `StepPreview`, `StepResult`, home): do not run them in parallel. S3 → S6 is sequential for the same reason (S6 adds a note to the home card built in S3).
- S4 and S1b both touch the app shell and `theme.css`: merge S1b first if both are in flight.

## Shared conventions (apply to every slice)

- **Layout**: Svelte 5 + Vite + TypeScript at repo root (`src/`), Rust in `src-tauri/`. Test with `npm test` (vitest) and `cargo test` (in `src-tauri/`). Builds: Windows `npm run portable` → single `.exe` in `src-tauri\target\release\`; macOS `.app`/`.dmg` (ad-hoc signed, not notarized) and Linux AppImage come from CI (`.github/workflows/ci.yml`, matrix windows/macos/linux). No installer. Agent guide: `/CLAUDE.md` and `docs/agents/`.
- **Rust**: one directory per vertical slice in `src-tauri/src/` (`syncs`, `wizard`, `run`, `history`, `engine` with `robocopy`, `rsync`, `preview_parser`, and `shared`), each with its own `commands.rs`, exposed through async `#[tauri::command]`s (blocking work in `tauri::async_runtime::spawn_blocking`) and events to the UI. Unit tests are `#[test] fn snake_case_name()` next to the module; integration tests with temp dirs go in `src-tauri/tests/`.
- **Svelte**: one component per wizard step in `src/wizard/`, other components in their vertical-slice folder under `src/` (`syncs/`, `run/`, `history/`, `schema/`) or `src/shared/`; talks to Rust only via `invoke` and events.
- **Domain terms** (use them in code, Rust and TS; full list in `docs/agents/glossary.md`): `Sync` (saved A→B pair: name, source, destination, mode), `SyncMode` (`Copy` | `Mirror`), `Preview` (lists New / Updated / Deleted), `Run` (one execution, recorded in `History`).
- **Engine**: `engine/mod.rs` is the boundary: robocopy on Windows (`engine/robocopy.rs`, facts in `docs/agents/robocopy.md`), rsync on macOS/Linux (`engine/rsync.rs`, `docs/agents/rsync.md`), chosen by `cfg`. Decide success by exit code and classify output by codes/paths, never by localized or English labels. Platform-only code is `cfg`-gated.
- **Messages**: Rust owns every user-facing message (`Explanation {title, advice}`, made from a `Cause`); `history.jsonl` stores causes, not prose.
- **Storage** in the platform data dir, never next to the app: `%LOCALAPPDATA%\ColomboSync\`, `~/Library/Application Support/ColomboSync`, `$XDG_DATA_HOME/ColomboSync` or `~/.local/share/ColomboSync` (read the env var / home, not Tauri's app dir): `syncs.json`, `history.jsonl`, `logs/<yyyyMMdd-HHmmss>-<sync>.log`. Serialization via `serde` / `serde_json`; files written atomically.
- **Look**: "Eclissi" logo, dark Apple-style theme in teal with a light variant, system font (Nunito removed), rounded corners. Reference for look: `docs/design/`; `prototype.html` stays the reference for flow and exact copy. Colors as CSS custom properties in one place (`src/theme.css`). Schema = one Svelte component `SchemaView.svelte` (inline SVG) in the side panel: **visible on home and in the wizard/run/result; the Storico tab shows the history legend instead**.
- **UI language**: Italian, exactly the wording of the prototype unless a slice says otherwise. New copy is listed in the slice report and recorded in `scope.md` "Decisions during implementation".
- **Review and CI**: every slice branch passes CI on all three OS and `/code-review` before merge (see `docs/agents/xp.md`, `docs/agents/github.md`).

---

## S1 · Create a sync with the wizard, preview it on the schema and run it
**Status:** done (commits `92a94a1` … `d23a151`)
**Why:** walking skeleton – proves wizard → robocopy preview → schema → real copy → history, through every layer, with the safe default mode.
**Size:** M
**Blocked by:** –

### Outcome
Wizard (Da dove → Dove → Opzioni → Anteprima), `SchemaView`, robocopy runner (preview + real run), `history.jsonl`, app shell with tabs. Copy mode only; no saved syncs, no mirror, "Sto copiando…" without real progress.

### Acceptance criteria
- Given the app is open, when the user picks A (`D:\Foto`), picks B (`E:\Backup`) and clicks "Avanti" through "Opzioni", then the preview shows "Da copiare (nuovi)" and "Da aggiornare" with counts and the schema shows the same counts plus "Nessun file viene cancellato in B".
- Given B contains files that are not in A, when the preview runs in Copy mode, then those files are not counted or listed anywhere and are still in B after the run.
- Given the preview has nothing to do, then the screen shows "B è già aggiornato" and no "Sì, avvia" button.
- Given the preview, when the user clicks "Sì, avvia", then robocopy runs with `/E` (never `/MIR`/`/PURGE`), the result says "Fatto: N file copiati, M aggiornati in …", one line is appended to `history.jsonl` and the robocopy log exists in `logs\`.
- Given robocopy exits with 1, 2 or 3, then the outcome is recorded as OK; given exit ≥ 8, the outcome is "Errore" (raw message for now; plain language in S6).

---

## S1b · Bring the app to the new prototype look and wizard details
**Status:** done (2026-11-04, merged on main; manual flow on the exe not yet verified)
**Why:** the user approved an updated prototype (2026-11-04) with a new look and a click-first folder picker; the real-user checkpoint after this slice must test the real look, not the old one.
**Size:** S/M
**Blocked by:** S1

### Acceptance criteria
- Given the app opens, then the top bar shows the brand "ColomboSync" with the subtitle "Copie di sicurezza senza sorprese" and a theme switch Auto / Chiaro / Scuro; cards use rounded corners (`--r` 22px, `--r-s` 14px), inputs have borders, surfaces have soft shadows.
- Given the user picks "Scuro", then the dark tokens of the prototype apply immediately; after restarting the app the choice is still there; "Auto" follows the Windows setting.
- Given the computer has no network, then the text is rendered in Nunito (the font is bundled in the exe; no request to any external host); if the font file fails to load the text falls back to Segoe UI.
- Given step "Da dove" (and "Dove"), then the user sees the click-first picker: "Clicca sulla cartella da copiare (A)" / "Clicca sulla cartella dove mettere la copia (B)"; at the root the header says "Dischi del computer: scegline uno", after choosing a folder it says "Cartella scelta: <b>X</b>". "Preferisci scrivere il percorso a mano?" is collapsed; when the user types a path there and presses Enter, it behaves as "Avanti"; when a step error appears, the section opens and the input gets focus.
- Given the preview has something to do, then no wizard footer is shown after the scan: only "Sì, avvia", "No, salva e basta" and the link "Indietro, cambio qualcosa"; given "B è già aggiornato", the screen has an "Indietro" link.

### Implementation notes
- Touch: `src/theme.css` (light + dark tokens copied from the prototype), app shell (top bar, theme switch; choice kept in `localStorage`, `Auto` = `prefers-color-scheme`), `FolderPicker.svelte` (today it opens the native dialog), `StepSource`, `StepDestination`, `StepPreview`, `Wizard` footer. Tokens stay in `src/theme.css`; no inline colors.
- Font: put the Nunito woff2 in `src/` (e.g. `src/assets/fonts/`), declare `@font-face` in `theme.css` with Vite-resolved URLs so the build inlines/bundles it; `font-family: Nunito, "Segoe UI", sans-serif`. No Google Fonts `<link>` or `@import`.
- Picker needs a new Rust module `folder_browser` with a command that lists drives (root) or the subfolders of a path (directories only, errors as plain Italian), exposed through `#[tauri::command]`. Test: `#[test] fn lists_only_directories()` with a temp dir. The picker keeps UNC paths working through the manual input.
- Tests: vitest component tests for the picker header states, Enter in the manual input, theme choice persisted (stub `localStorage`), preview footer states. The look itself is checked by eye against `prototype.html`.
- Do not touch: Rust robocopy/history logic, mirror (S2), saved syncs (S3).

> **Checkpoint 1 (after S1b, before S2):** a real non-technical user tries the portable exe in Copy mode with 2 tasks: (1) create a copy, (2) read the preview and say what will happen. Fix findings before S2.

---

## S2 · Turn on mirror from advanced options, with warning and explicit consent
**Status:** done (2026-11-04, merged on main; manual flow on the exe not yet verified)
**Why:** the riskiest UX – deleting files on B must be impossible to do by accident.
**Size:** M
**Blocked by:** S1b

### Acceptance criteria
- Given step "Opzioni", then "Opzioni avanzate" is collapsed and mirror is off; when the user turns on "Rendi B identico ad A (specchio)", then the red warning box appears immediately and the schema switches to "Specchio" with B and the arrow in red and an "Eliminati da B" counter.
- Given mirror is on and the preview finds 12 files only in B, then a red group "Da cancellare in B" lists them and "Sì, avvia" stays disabled until "Ho capito: 12 file verranno cancellati da B" is checked.
- Given mirror is on and the preview finds 0 extra files, then no consent checkbox is shown and the text says "In B non ci sono file in più: non verrà cancellato niente."
- Given a mirror run completes, then the history line records mode `Mirror` and the deleted count, and robocopy was called with `/MIR`.
- Given mirror is on and A does not exist or has no entries, or the preview counts more deleted files than new + updated, then the preview shows the warning "A sembra vuota o scollegata: con lo specchio verrebbe svuotato B. Controlla che il disco di A sia collegato." and offers **no** "Sì, avvia" at all (protects against an unplugged disk wiping B).
- Given B equal to A or inside A (case-insensitive, trailing slashes, UNC), when the user clicks "Avanti" on step "Dove", then the wizard blocks with "La destinazione è la stessa cartella da copiare. Scegline un’altra." / "La destinazione non può stare dentro la cartella da copiare."

### Implementation notes
- Preview in mirror uses `/MIR /L`; extras (paths under B) become Deleted.
- `mirror_guard` does **not** walk the tree: "A is empty" = `A` missing or `read_dir(A)` yields no entry (cheap); "deletes exceed what A brings" is approximated by `deleted > new + updated` from the preview counts. Accepted risk: it can block a legitimate big cleanup (see scope.md); an exact guard on the full file count of A is in "Later".
- `path_rules` module (moved here from S6): pure function on two path strings, no filesystem access; the Wizard calls it through a command so the rule lives in Rust.
- Consent is per run: re-opening the preview always unchecks it.
- Tests: `#[test] fn mirror_mode_uses_mir()` (robocopy); `extra_files_count_as_deleted_only_in_mirror()` (preview_parser); `mirror_guard` unit tests `blocks_when_source_missing_or_empty()` and `blocks_when_deleted_exceed_new_plus_updated()`; `path_rules` unit tests `destination_inside_source_is_rejected()` and `destination_equal_to_source_is_rejected()` (case-insensitive, trailing slashes, UNC); integration (`src-tauri/tests/`) `mirror_deletes_extra_files_in_destination()`.

> **Checkpoint 2 (after S2):** a real non-technical user, mirror task only: turn on mirror, then read the preview and say what gets deleted. Fix wording/layout findings before going further.

---

## S3 · Name and save syncs, see them on the home screen and run one again
**Status:** done (2026-11-04, merged on main; manual flow on the exe not yet verified)
**Why:** the repeat job – after the first time, the user just clicks "Controlla e avvia".
**Size:** M
**Blocked by:** S1b

### Acceptance criteria
- Given the wizard, then a "Come la chiamo?" step (prefilled "Copia di <last folder of A>") sits before "Opzioni"; an empty name blocks "Avanti". The sync is saved to `syncs.json` when the run starts or on "No, salva e basta".
- Given two saved syncs, when the app opens, then the home "Le tue sincronizzazioni" (with the lead text of the prototype) lists one card per sync showing A → B with drive-kind icons, the mode chip, and "Ultima volta: 02/11/2026 21:30" with an OK/Errore chip, or "Mai avviata"; selecting a card shows its A and B in the schema, with the caption 'Premi "Controlla e avvia": prima ti mostro cosa cambierà, poi decidi tu.'
- Given a saved sync, when the user clicks "Controlla e avvia" (it reads "Riprova" when the last outcome is Errore), then the app goes straight to the preview with fresh counts; it never runs without showing the preview.
- Given `syncs.json` is missing or unreadable, then the home shows an empty state with the dashed card "Nuova sincronizzazione" and the app does not crash; with saved syncs the same dashed card sits below the list.

### Implementation notes
- Home is the main view; the wizard opens from it and returns to it. Module `sync_store` (Rust) reads/writes `syncs.json`.
- The plain-language error note on the card is S6; S3 shows only the outcome chip.
- Tests: `#[test] fn round_trips_syncs()` and `corrupt_file_returns_empty_list()` in `sync_store`; vitest for card states (Mai avviata / OK / Errore → "Riprova").

---

## S4 · Browse the history of runs and open a run's log
**Status:** done (2026-11-04, merged on main; manual flow on the exe not yet verified)
**Why:** the user explicitly asked for a history; it is also how they check that last night's copy happened.
**Size:** S
**Blocked by:** S1 (can run in parallel to S3)

### Acceptance criteria
- Given runs in `history.jsonl`, when the user opens the "Storico" tab, then the lead says "Tutte le copie fatte, dalla più recente. Ogni riga ha il suo registro completo." and runs are listed newest first with date and time, sync name, mode chip, Nuovi / Aggiornati / Cancellati, durata and outcome chip (OK / Errore / Annullato).
- Given the Storico tab, then the side panel shows "Come leggere lo storico" instead of the schema: counters "Copie fatte" and "Riuscite", a legend for the OK / Errore / Annullato / Specchio chips (texts as in the prototype) and "Ultima copia riuscita: <when>".
- When the user clicks "Registro" on a run, then a modal "Registro – <name> (<when>)" shows the robocopy log read-only, with "Chiudi" (Esc also closes); if the log file was deleted, it says "Il registro di questa copia non è più disponibile."
- Given a malformed line in `history.jsonl`, then that line is skipped and the others are shown.

### Implementation notes
- Command to read a run's log file (UTF-16LE with BOM, via the `history`/`storage` module). The side panel switches by tab in the app shell; `SchemaView` is not rendered on Storico.
- Tests: `#[test] fn reads_newest_first()` and `skips_malformed_lines()` in `history`; vitest for legend counters and the modal closing on Esc.

---

## S5 · Follow the copy live on the schema and stop it safely
**Status:** done (2026-11-04, merged on main; manual flow on the exe not yet verified)
**Why:** a long copy with no feedback looks like a crash to a non-technical user.
**Size:** M
**Blocked by:** S1

### Acceptance criteria
- Given a run of 151 files, while it runs, then the screen says "Sto copiando <name>…" with a big percentage, "N di 151 file", "Mancano circa N secondi", the current file name and an animated schema arrow; screen readers get an aria-live announcement every 10%.
- When the user clicks "Ferma la copia", then the inline question "Vuoi fermare la copia? I file già copiati restano in B." appears with "Sì, ferma" / "No, continua"; on "Sì, ferma" the robocopy process is killed, the result says "Annullato" and history records outcome `Annullato`.
- When the user closes the window during a run, then the same confirmation is asked before exiting.
- Given a run that ends in Errore or Annullato after copying 40 files, then the result and the history row say "40 file copiati (parziale)"; given a run that ends OK, history keeps the preview counts (new / updated).

### Implementation notes
- Progress: `/UNILOG` is written in bursts, so read redirected stdout line by line (decode the console OEM code page or `/UNICODE`; decide while implementing), keeping `/UNILOG` for the history log via `/TEE`. Total comes from the preview (new + updated). Emit progress as Tauri events; ETA from rate so far.
- Stop = kill the robocopy child process (it spawns no children); never leave it running after the app exits. Closing the window: Tauri `CloseRequested` → `prevent_close` + event to the UI, exit only after confirmation.
- Real counts: on Errore/Annullato count the file lines in the run's own `/UNILOG` and store a single partial total. **Do not reuse the preview parser's classification for the run log**: it labels a file Updated when `B\<relative>` exists, and after a copy every file exists in B, so everything would be counted as Updated. No new/updated split for partial runs (in "Later").
- `history::Run` gets an optional `partial: Option<usize>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`); existing `history.jsonl` lines must still load. The Storico row (S4) shows "N file copiati (parziale)" when it is set.
- Tests: `#[test] fn counts_file_lines_in_run_log()`; `run_without_partial_field_still_loads()` (history); vitest for the confirm question and the percentage/ETA texts.

---

## S6 · Explain problems in plain Italian
**Status:** done (2026-11-04, merged on main; manual flow on the exe not yet verified)
**Why:** "ERRORE 53" means nothing to the user; every failure must say what happened and what to do.
**Size:** S
**Blocked by:** S1, S3, S4 (S3: shares the home card and Wizard/StepPreview files, run after it; S4: "Apri il registro" opens its log viewer)

### Acceptance criteria
- Given the network share is unreachable at preview or during the run, then the result says "Non completato: la cartella di rete \\NAS non è raggiungibile. Controlla la connessione e riprova." with "Riprova" and "Apri il registro".
- Given A does not exist (disk unplugged), then the preview says "Non trovo la cartella A (D:\Foto). Il disco è collegato?" and offers no start button.
- Given the app runs elevated, then a banner warns "Le unità di rete potrebbero non essere visibili: apri ColomboSync senza 'Esegui come amministratore'."
- Given the last run of a saved sync failed, then its home card shows the note "La cartella di rete \\NAS non era raggiungibile. Controlla che sia collegato e riprova."; given it was stopped, the note is "La copia era stata fermata a mano: puoi riprenderla quando vuoi."

### Implementation notes
- Module `error_messages`: map Windows error codes found in the log (`ERROR 53`, `67`, `2`, `3`, `5`) to messages **by code, never by label**; anything else → generic "Non completato" + log link. Disk vs network wording follows the path (UNC → "cartella di rete", drive letter → "disco"), as in the prototype.
- Path rules are not here any more: they moved to S2.
- Tests: `#[test]` one case per mapped code in `error_messages`; vitest for the card notes and the elevated banner.

---

## S7 · Build and run on macOS and Linux with rsync
**Status:** done (2026-11-04, merged on main; CI green on Windows, macOS, Linux; apps not opened by hand yet)
**Why:** the user wants the same app on Mac and Linux; robocopy only exists on Windows.
**Size:** L → split while implementing if needed (engine first, then platform details, then CI bundles)
**Blocked by:** S6

### Acceptance criteria
- Given macOS or Linux, when the user previews A → B in Copy mode, then rsync runs as a dry run and the preview shows the same New / Updated counts and lists as on Windows; nothing in B is touched.
- Given Mirror mode on macOS/Linux, then rsync uses `--delete` only in Mirror, files only in B are listed as "Da cancellare in B", and consent + MirrorGuard behave exactly as on Windows (integration test with temp dirs on the CI runners).
- Given a run on macOS/Linux, then progress, "Ferma la copia" (kill rsync), partial counts, history and logs work as on Windows; data lives in `~/Library/Application Support/ColomboSync` (macOS) or `$XDG_DATA_HOME/ColomboSync` / `~/.local/share/ColomboSync` (Linux).
- Given an rsync failure (exit 23, 24, 10–12, 3, 13…), then the plain-Italian messages of S6 are shown (mapped by exit code, never by English text).
- Given a push to main, then GitHub Actions builds and tests on windows-latest, macos-14 and ubuntu-latest and uploads `colombo-sync.exe`, a zipped `.app`/`.dmg` (ad-hoc signed) and an AppImage.

### Implementation notes
- Riskiest assumption: recent macOS ships `openrsync`, not GNU rsync 3.x. Spike first on the macos runner: `rsync --version`, and whether `rsync -a --dry-run --itemize-changes [--delete] A/ B/` gives `>f+++++++++` (new), `>f.st……` (updated), `*deleting` (extra). Fallback: `--out-format='%i %n'`. Classify by itemize code, never by message text.
- Engine: two real implementations now (robocopy, rsync) → one engine boundary (enum or trait) chosen by `cfg(target_os)`; the rest of the app (preview_parser output shape `Preview`, runner, MirrorGuard, PreviewMemo, history) stays shared.
- `cfg(windows)` gating for the Windows-only parts: `MetadataExt` in folder_browser, `MultiByteToWideChar` in progress, token elevation, `CREATE_NO_WINDOW`, `%LOCALAPPDATA%`, A:–Z: drive probing. Unix counterparts: folder browser roots `/Volumes` (macOS) and `/media/$USER`, `/mnt`, home (Linux); elevation = effective uid 0; path_rules for `/`-rooted, case-sensitive paths (macOS default volumes are case-insensitive: compare case-insensitively there).
- Bundles: macOS `tauri build --bundles app,dmg` with `bundle.macOS.signingIdentity = "-"`; Linux `--bundles appimage` (CI deps: libwebkit2gtk-4.1-dev, libappindicator3-dev, librsvg2-dev, patchelf).
- UI copy that is Windows wording ("Esegui come amministratore", "Cestino", drive letters in examples): flag per platform; new Italian copy must be listed in the report for the user.

---

## S8 · Release pipeline
**Status:** done (2026-11-04, merged on main; dry run and `v0.1.0-rc.1` draft green, version-mismatch tag correctly failed; first beta `v0.1.0-beta.1`)
**Why:** one tag gives tested, checksummed binaries the owner can hand out to users.
**Size:** M
**Blocked by:** S7
**Spec:** `docs/release-plan.md` (decisions taken by the owner: distribution by hand, universal macOS dmg, Linux AppImage only).

### Acceptance criteria
- Given a `v0.1.0-rc.1` tag, then `.github/workflows/release.yml` creates a **draft** release (pre-release because the version starts with `0.` or has a `-` suffix) with `ColomboSync-<ver>-windows-x64.exe`, `ColomboSync-<ver>-macos-universal.dmg`, `ColomboSync-<ver>-linux-x86_64.AppImage`, `SHA256SUMS.txt` and notes (commits since the previous tag grouped feat/fix + the Italian install notes).
- Given the tag does not match `package.json`, `tauri.conf.json` and `Cargo.toml`, or those three differ from each other, then the run fails before any build.
- Given any test/lint job red on any OS, then no release is created.
- Given the macOS build, then the dmg contains a universal binary (`lipo -info` shows arm64 and x86_64, asserted in CI); the AppImage is built on ubuntu-22.04.
- Given a manual run (`workflow_dispatch`), then the three files are built and uploaded as workflow artifacts and **no release** is created.
- Actions in both workflows run on Node 24; the CI Linux job also uses ubuntu-22.04.
- `docs/agents/github.md` has a "Releasing" section (bump, tag, watch, smoke checklist, publish by hand).

---

## Later (cut from this round)
- Scheduling (Task Scheduler) – manual start only for now.
- Exclusion filters (file types, folders).
- SSH / remote destinations.
- Installer, auto-update, tray icon, notifications.
- Protecting newer files in B (`/XO`) – today A always wins, as in a normal copy; revisit if users complain.
- Two-way sync; several sources into one destination.
- Partial run counts split into new/updated (today a single partial total).
- Exact MirrorGuard based on the full file count of A.
