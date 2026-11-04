# Release plan – ColomboSync binaries

**Status:** agreed 2026-11-04.

## Goal
One tag → one GitHub Release with tested, checksummed binaries for Windows, macOS and Linux, plus Italian install notes a non-technical user can follow. No installer, no auto-update, no paid signing (decided).

## What ships
| OS | File | Built on | Notes |
|---|---|---|---|
| Windows 10/11 x64 | `ColomboSync-<ver>-windows-x64.exe` | `windows-latest` | Portable exe (`tauri build --no-bundle`). Needs WebView2 (preinstalled on 10/11). Unsigned → SmartScreen warning. |
| macOS 11+ (Intel + Apple Silicon) | `ColomboSync-<ver>-macos-universal.dmg` | `macos-14` (pinned: `macos-latest` jobs stayed queued with no runner) | `--target universal-apple-darwin` so Intel Macs work too (today CI builds arm64 only). Ad-hoc signed, not notarized → right-click → Apri the first time. Uses the system `openrsync`. |
| Linux x86_64 | `ColomboSync-<ver>-linux-x86_64.AppImage` | `ubuntu-22.04` | Build on 22.04, not `ubuntu-latest` (24.04): an AppImage only runs on distros with a glibc at least as new as the build machine. Needs `rsync` installed (the app says so in Italian if missing). |
| all | `SHA256SUMS.txt` | release job | Checksums of the three files. |

## Versioning
- SemVer. First public build **0.1.0**, published as a **pre-release** (beta) until the two real-user checkpoints in the backlog are done; **1.0.0** after them.
- One version in three places: `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`. CI fails if they differ or don't match the tag (`v0.1.0` ↔ `0.1.0`).
- Bump with one small script/commit `chore(release): 0.1.1`, then tag.

## Pipeline (`.github/workflows/release.yml`)
1. **Trigger:** push of a tag `v*` (plus `workflow_dispatch` for a dry run that uploads artifacts but creates no release).
2. **Gate:** reuse the CI matrix: fmt, clippy, cargo test, npm check/test on all three OS. Any red → no release.
3. **Version check:** tag == the three version fields.
4. **Build** per OS (table above), rename files to the release names.
5. **Release job:** download the three artifacts, write `SHA256SUMS.txt`, create a **draft** GitHub Release with release notes from the commits since the previous tag (`feat(…)`/`fix(…)` grouped) + the Italian install notes below.
6. **Publish by hand** after the smoke checklist: a person flips draft → published (pre-release for 0.x).

Housekeeping in the same change: bump `actions/checkout`, `setup-node`, `upload-artifact` to the versions that run on Node 24 (CI warns today).

## Smoke checklist before publishing (manual, ~15 min per OS)
On a clean machine or VM per OS, with the release files (not local builds):
1. Opens (after the OS warning), Eclissi logo and dark/light switch visible.
2. New sync A → B on a USB stick: preview shows counts, "Sì, avvia" copies, Storico has the run, Registro opens.
3. Mirror: extra file in B is listed in red, "Sì, avvia" disabled until consent, file is deleted after the run.
4. Unplug B, re-run: plain Italian error, run recorded as Errore.
5. Close the window during a copy: the stop question appears.
Record the result as a comment on the draft release. Any failure → fix, new patch tag.

## Install notes shipped with every release (Italian, user-facing)
- **Windows:** scarica `ColomboSync-…-windows-x64.exe` e mettilo dove vuoi (per esempio sul Desktop). La prima volta Windows può dire "Windows ha protetto il PC": clicca **Ulteriori informazioni** → **Esegui comunque**.
- **Mac:** apri il `.dmg` e trascina ColomboSync in Applicazioni. La prima volta: tasto destro sull'app → **Apri** → **Apri** (su macOS recenti: Impostazioni di Sistema → Privacy e sicurezza → **Apri comunque**).
- **Linux:** scarica l'`.AppImage`, rendilo eseguibile (tasto destro → Proprietà → "Consenti l'esecuzione", oppure `chmod +x`) e aprilo. Serve `rsync` installato.
- **Aggiornare:** scarica la nuova versione e sostituisci la vecchia. Sincronizzazioni e storico restano (sono salvati nella cartella dell'utente, non accanto al programma).

## Rollback
Releases are never edited in place. A bad release is marked "non usare" in its notes and the previous one is re-marked latest; the fix ships as a new patch version. User data format (`syncs.json`, `history.jsonl`) stays backward compatible (serde defaults), so going back one version is safe.

## Decisions (owner, 2026-11-04)
1. **Distribution: by hand.** The repo stays private; the release job creates a **draft** release there, the owner downloads the files and shares them privately (e.g. a shared drive), together with the Italian install notes.
2. **macOS: universal dmg** (Apple Silicon M1/M2/… and Intel in one file).
3. **Linux: AppImage only.**

## Implementation slice (for the backlog)
**S8 · Release pipeline** – Blocked by: S7. AC: a `v0.1.0-rc.1` tag produces a draft release with the three renamed binaries + `SHA256SUMS.txt` + notes; version mismatch fails the run; the macOS dmg opens on Intel and Apple Silicon; the AppImage runs on Ubuntu 22.04; dry-run via `workflow_dispatch` creates no release.
