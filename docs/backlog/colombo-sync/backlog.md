# Backlog – ColomboSync

Scope: ./scope.md · Prototype (flow, copy, look): ./prototype.html · Implementer: agent alone · Tracker: none (markdown only)

Order is dependency order. A slice with no shared "Blocked by" can start in parallel.

```
S1 ──┬── S2 (mirror)
     ├── S3 (saved syncs) ── S5 (progress + cancel)
     ├── S4 (history)
     └── S6 (plain-language errors)
```

## Shared conventions (apply to every slice)

> **Stack changed 2026-11-03 (user):** Tauri 2 + Svelte 5 replaces WinForms/net48. Where a slice's notes still name C#/WinForms things, translate with the table below. Agent guide: `/CLAUDE.md` and `docs/agents/`.

- **Layout**: Svelte 5 + Vite + TypeScript at repo root (`src/`), Rust in `src-tauri/`. Test with `npm test` (vitest) and `cargo test` (in `src-tauri/`). Portable build: `npm run portable` → single `.exe` in `src-tauri\target\release\`, no installer (WebView2 assumed present).
- **Domain terms** (use them in code, Rust and TS): `Sync` (saved A→B pair: name, source, destination, mode), `SyncMode` (`Copy` | `Mirror`), `Preview` (lists New / Updated / Deleted), `Run` (one execution, recorded in `History`).
- **Engine (Rust)**: module `robocopy` builds arguments, runs `%SystemRoot%\System32\robocopy.exe` hidden (`CREATE_NO_WINDOW`), maps exit codes (0–7 success, ≥ 8 failure). Always `/R:1 /W:1`. Never parse localized labels (see scope.md, Decisions). `/UNILOG` is UTF-16LE with BOM.
- **Storage (Rust)** in `%LOCALAPPDATA%\ColomboSync\` (read the `LOCALAPPDATA` env var, not Tauri's app dir): `syncs.json`, `history.jsonl`, `logs\<yyyyMMdd-HHmmss>-<sync>.log`. Serialization via `serde_json`.
- **Look**: copy tokens (light theme) and texts from `prototype.html`; tonal fills, no borders, square corners, Segoe UI. Colors as CSS custom properties in one place (`src/theme.css`). Schema = one Svelte component `SchemaView.svelte` (inline SVG), docked right, visible on every screen.
- **UI language**: Italian, exactly the wording of the prototype unless a slice says otherwise.

| Backlog says (C#) | Means (Tauri) |
|---|---|
| class `Robocopy`, `PreviewParser`, `History`, `SyncStore`, `ErrorMessages`, `PathRules`, `MirrorGuard` | Rust module `robocopy`, `preview_parser`, `history`, `sync_store`, `error_messages`, `path_rules`, `mirror_guard` in `src-tauri/src/` |
| `XxxTests.Some_case` (xUnit) | `#[test] fn some_case()` in that module's `tests` (integration with temp dirs: `src-tauri/tests/`) |
| Form / step / `SchemaView` control | Svelte component in `src/lib/` (one per wizard step) |
| `FolderBrowserDialog` | `@tauri-apps/plugin-dialog` `open({ directory: true })` + editable path input |
| background thread | async Tauri command (`tauri::async_runtime::spawn_blocking`), events to UI |
| `DataContractJsonSerializer` | `serde` / `serde_json` |
| "exe < 1 MB" | dropped: the release exe is what ships, no installer |

---

## S1 · Create a sync with the wizard, preview it on the schema and run it
**Why:** walking skeleton – proves wizard → robocopy preview → schema → real copy → history, through every layer, with the safe default mode.
**Size:** M
**Blocked by:** –

### Acceptance criteria
- Given the app is open, when the user picks A (`D:\Foto`), picks B (`E:\Backup`) and clicks "Avanti" through "Opzioni", then the preview shows "Da copiare (nuovi)" and "Da aggiornare" with counts and the schema shows the same counts plus "Nessun file viene cancellato in B".
- Given B contains files that are not in A, when the preview runs in Copy mode, then those files are not counted or listed anywhere and are still in B after the run.
- Given the preview has nothing to do, then the screen shows "B è già aggiornato" and no "Sì, avvia" button.
- Given the preview, when the user clicks "Sì, avvia", then robocopy runs with `/E` (never `/MIR`/`/PURGE`), the result says "Fatto: N file copiati, M aggiornati in …", one line is appended to `history.jsonl` and the robocopy log exists in `logs\`.
- Given robocopy exits with 1, 2 or 3, then the outcome is recorded as OK; given exit ≥ 8, the outcome is "Errore" (raw message for now; plain language in S6).

### Implementation notes
- Touch: create the solution; `Wizard` form (steps Da dove → Dove → Opzioni → Anteprima; the "Nome" step and saving come in S3; folder choice via `FolderBrowserDialog` + editable path box accepting UNC), `SchemaView`, `Robocopy`, `PreviewParser`, `History`.
- Preview command: `robocopy "<A>" "<B>" /E /XX /L /R:1 /W:1 /NJH /NJS /NDL /NC /BYTES /FP /NP /UNILOG:<tmp>`; read the UTF-16 log. `/XX` keeps extras in B out of the output in Copy mode. Classify each line by full path: under B → extra (Deleted, mirror only, S2); under A and `B\<relative>` exists → Updated; else → New.
- Run command: same without `/L`, with `/UNILOG:<logs path>`. Run on a background thread; UI shows a simple "Sto copiando…" (real progress is S5).
- No `syncs.json` yet: "No" on "Avviare adesso?" just returns to the start. The history line stores A and B, with the sync name = "Copia di <last folder of A>".
- Tests: `PreviewParserTests.Classifies_new_updated_and_extra_by_path` (fixture from a real `/L` log, including a non-English label line); `RobocopyTests.Exit_codes_0_to_7_are_success`; `RobocopyTests.Copy_mode_never_uses_mir` (args builder); integration `RobocopyIntegrationTests.Copy_keeps_extra_files_in_destination` using temp folders.
- Do not touch: mirror UI (S2), home list (S3).
- Done when: tests pass, `bin\Release\ColomboSync.exe` is a single file < 1 MB and the manual flow above works on this machine.

---

## S2 · Turn on mirror from advanced options, with warning and explicit consent
**Why:** the riskiest UX – deleting files on B must be impossible to do by accident.
**Size:** M
**Blocked by:** S1

### Acceptance criteria
- Given step "Opzioni", then "Opzioni avanzate" is collapsed and mirror is off; when the user turns on "Rendi B identico ad A (specchio)", then the red warning box appears immediately and the schema switches to "Specchio" with B and the arrow in red and an "Eliminati da B" counter.
- Given mirror is on and the preview finds 12 files only in B, then a red group "Da cancellare in B" lists them and "Sì, avvia" stays disabled until "Ho capito: 12 file verranno cancellati da B" is checked.
- Given mirror is on and the preview finds 0 extra files, then no consent checkbox is shown and the text says "In B non ci sono file in più: non verrà cancellato niente."
- Given a mirror run completes, then the history line records mode `Mirror` and the deleted count, and robocopy was called with `/MIR`.
- Given mirror is on and the preview finds A empty (0 files) while B has files, or the files to delete are more than the files B would keep, then the preview shows a distinct warning "A sembra vuota o scollegata: con lo specchio verrebbe svuotato B. Controlla che il disco di A sia collegato." and offers **no** "Sì, avvia" at all (protects against an unplugged disk wiping B).

### Implementation notes
- Preview in mirror uses `/MIR /L`; extras (paths under B) become Deleted.
- Consent is per run: re-opening the preview always unchecks it.
- Tests: `RobocopyTests.Mirror_mode_uses_mir`; `PreviewParserTests.Extra_files_count_as_deleted_only_in_mirror`; `MirrorGuardTests.Blocks_start_when_source_empty_or_deletes_exceed_kept`; integration `Mirror_deletes_extra_files_in_destination`.

---

## S3 · Name and save syncs, see them on the home screen and run one again
**Why:** the repeat job – after the first time, the user just clicks "Avvia".
**Size:** M
**Blocked by:** S1

### Acceptance criteria
- Given the wizard, then a "Come la chiamo?" step (prefilled "Copia di <last folder of A>") sits before "Opzioni"; an empty name blocks "Avanti". The sync is saved to `syncs.json` when the run starts or on "No, salva e basta".
- Given two saved syncs, when the app opens, then the home "Le tue sincronizzazioni" lists both with name, A → B, mode chip and last outcome ("Ultima volta: 02/11/2026 21:30" + OK/Errore), and selecting one shows its A and B in the schema.
- Given a saved sync, when the user clicks "Avvia", then the app goes straight to the preview (step 5) with fresh counts; it never runs without showing the preview.
- Given `syncs.json` is missing or unreadable, then the home shows an empty state with "+ Nuova sincronizzazione" and the app does not crash.

### Implementation notes
- Home is the main form; the wizard opens from it and returns to it.
- Tests: `SyncStoreTests.Round_trips_syncs`; `SyncStoreTests.Corrupt_file_returns_empty_list`.

---

## S4 · Browse the history of runs and open a run's log
**Why:** the user explicitly asked for a history; it is also how they check that last night's copy happened.
**Size:** S
**Blocked by:** S1

### Acceptance criteria
- Given runs in `history.jsonl`, when the user opens the "Storico" tab, then runs are listed newest first with date and time, sync name, mode chip, Nuovi / Aggiornati / Cancellati, durata and outcome chip (OK / Errore / Annullato).
- When the user clicks "Registro" on a run, then the robocopy log of that run opens in an in-app read-only window; if the log file was deleted, the window says "Il registro di questa copia non è più disponibile."
- Given a malformed line in `history.jsonl`, then that line is skipped and the others are shown.

### Implementation notes
- Tests: `HistoryTests.Reads_newest_first`; `HistoryTests.Skips_malformed_lines`.

---

## S5 · Follow the copy live on the schema and stop it safely
**Why:** a long copy with no feedback looks like a crash to a non-technical user.
**Size:** M
**Blocked by:** S3

### Acceptance criteria
- Given a run of 151 files, while it runs, then the schema arrow is animated, a progress bar shows "N di 151 file", the current file name is shown, and the counters grow.
- When the user clicks "Annulla", then an inline question "Vuoi fermare la copia? I file già copiati restano in B." appears; on "Sì, ferma" robocopy is terminated, the result says "Annullato" and history records outcome `Annullato` with partial counts.
- When the user closes the window during a run, then the same confirmation is asked before exiting.

### Implementation notes
- Progress: `/UNILOG` is written in bursts, so read redirected stdout line by line instead (`/UNICODE` + UTF-16 decoding, or the console OEM code page – decide while implementing), keeping `/UNILOG` for the history log via `/TEE`. Total comes from the preview.
- Cancel = `Process.Kill()` on robocopy (it spawns no children); never leave it running after the app exits.

---

## S6 · Explain problems in plain Italian
**Why:** "ERRORE 53" means nothing to the user; every failure must say what happened and what to do.
**Size:** S
**Blocked by:** S1

### Acceptance criteria
- Given B is inside A or equal to A, when the user clicks "Avanti" on step 2, then the wizard blocks with "La destinazione non può stare dentro la cartella da copiare." / "La destinazione è la stessa cartella da copiare. Scegline un'altra."
- Given the network share is unreachable at preview or during the run, then the result says "Non completato: la cartella di rete \\NAS non è raggiungibile. Controlla la connessione e riprova." with "Riprova" and "Apri il registro".
- Given A does not exist (disk unplugged), then the preview says "Non trovo la cartella A (D:\Foto). Il disco è collegato?" and offers no start button.
- Given the app runs elevated, then a banner warns "Le unità di rete potrebbero non essere visibili: apri ColomboSync senza 'Esegui come amministratore'."

### Implementation notes
- Map robocopy Windows error codes found in the log (`ERROR 53`, `67`, `2`, `3`, `5`) to messages; anything else → generic "Non completato" + log link.
- Tests: `ErrorMessagesTests` with one case per mapped code; `PathRulesTests.Destination_inside_source_is_rejected` (case-insensitive, trailing slashes, UNC).

---

## Later (cut from this round)
- Dark theme following Windows settings (prototype has one; not needed for MVP).
- Scheduling (Task Scheduler) – manual start only for now.
- Exclusion filters (file types, folders).
- SSH / remote, macOS/Linux with rsync.
- Installer, auto-update, tray icon, notifications.
- Protecting newer files in B (`/XO`) – today A always wins, as in a normal copy; revisit if users complain.
- Two-way sync; several sources into one destination.
