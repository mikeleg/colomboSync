---
name: build-portable
description: Build the ColomboSync portable Windows exe (no installer) and smoke-test that it starts. Use when asked to build, release, package or produce the exe, or to check the app still launches.
---

# Build the portable exe

```
npm ci            # first time / after package changes
npm run portable  # = tauri build --no-bundle
```

Output: `src-tauri\target\release\colombo-sync.exe` – that single file is the deliverable. No MSI/NSIS. Needs WebView2 (present on Windows 10/11).

## Smoke test
```powershell
$p = Start-Process "src-tauri\target\release\colombo-sync.exe" -PassThru
Start-Sleep 5; if ($p.HasExited) { "FAILED exit $($p.ExitCode)" } else { "running"; Stop-Process $p.Id }
(Get-Item "src-tauri\target\release\colombo-sync.exe").Length / 1MB
```

## Release copy
Copy the exe to `dist-portable\ColomboSync.exe` if the user asks for a deliverable folder. Data lives in `%LOCALAPPDATA%\ColomboSync\`, never next to the exe.

## If it fails
- Linker errors → MSVC build tools missing; report, don't work around.
- Blank window → frontend not built: check `beforeBuildCommand` / `frontendDist` in `src-tauri/tauri.conf.json`.
