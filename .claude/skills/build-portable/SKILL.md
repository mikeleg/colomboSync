---
name: build-portable
description: Build the ColomboSync app for the current OS (Windows portable exe, no installer) and/or fetch the CI-built artifacts for the other OSes (macOS .app/.dmg, Linux AppImage). Use when asked to build, release, package or produce the exe/app/dmg/AppImage, or to check the app still launches.
---

# Build the app

The dev machine is Windows: it builds the exe itself. macOS and Linux bundles are built only by CI (`.github/workflows/ci.yml`) and downloaded.

## Windows (this machine)
```
npm ci            # first time / after package changes
npm run portable  # = tauri build --no-bundle
```
Output: `src-tauri\target\release\colombo-sync.exe` – that single file is the deliverable. No MSI/NSIS. Needs WebView2 (present on Windows 10/11). First build ~5 min.

### Smoke test
```powershell
$p = Start-Process "src-tauri\target\release\colombo-sync.exe" -PassThru
Start-Sleep 5; if ($p.HasExited) { "FAILED exit $($p.ExitCode)" } else { "running"; Stop-Process $p.Id }
(Get-Item "src-tauri\target\release\colombo-sync.exe").Length / 1MB
```
Data lives in `%LOCALAPPDATA%\ColomboSync\`, never next to the exe. Copy the exe to `dist-portable\ColomboSync.exe` only if the user asks for a deliverable folder.

## macOS and Linux (CI artifacts)
Push the branch (token recipe in `docs/agents/github.md`), wait for the run, then download into a new empty directory:
```
export GH_TOKEN=$(gh auth token -u mikeleg)
gh run list -R mikeleg/colomboSync --branch <branch> --limit 1
gh run watch <id> -R mikeleg/colomboSync --exit-status
gh run download <id> -R mikeleg/colomboSync -D <new-empty-dir>
```
Artifacts exist only for `slice/**` pushes, PRs and `workflow_dispatch` runs, not for pushes to main (to get them for main: `gh workflow run ci.yml -R mikeleg/colomboSync --ref main`).
- `colombo-sync-macos`: `ColomboSync-macos.zip` (the `.app`) and the `.dmg`, ad-hoc signed, **not notarized**: on first launch the user right-clicks → Apri. Data in `~/Library/Application Support/ColomboSync`.
- `colombo-sync-linux`: the `.AppImage` (`chmod +x` first). Data in `$XDG_DATA_HOME/ColomboSync` or `~/.local/share/ColomboSync`.
- `colombo-sync-windows`: the same exe `npm run portable` makes.

These can't be opened here: report "not opened by hand" for macOS/Linux. To build locally on a Mac/Linux box: `npx tauri build --bundles app,dmg` (macOS) / `--bundles appimage` (Linux; needs libwebkit2gtk-4.1-dev, libappindicator3-dev, librsvg2-dev, patchelf).

## If it fails
- Windows linker errors → MSVC build tools missing; report, don't work around.
- Blank window → frontend not built: check `beforeBuildCommand` / `frontendDist` in `src-tauri/tauri.conf.json`.
- CI build fails only on one OS → `gh run view <id> --log-failed -R mikeleg/colomboSync`; fix on a `slice/**` branch, don't push to `main` to test.
