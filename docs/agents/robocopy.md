# Robocopy – facts the engine relies on

Source: spike of 2026-11-03 (see `docs/backlog/colombo-sync/scope.md`, Decisions). Don't re-derive, don't contradict without a test proving otherwise.

- Binary: `%SystemRoot%\System32\robocopy.exe`, spawned with `CREATE_NO_WINDOW` (`std::os::windows::process::CommandExt::creation_flags(0x0800_0000)`).
- **Exit codes** are a bitmask: 1 = files copied, 2 = extra files in destination, 4 = mismatches. **0–7 = success, ≥ 8 = failure.** A preview returning 3 is not an error.
- **Always** `/R:1 /W:1` (default retries hang for days on a locked file).
- **Copy mode** = `/E`. Never `/MIR` or `/PURGE`. **Mirror mode** = `/MIR` (S2 only).
- **Preview** = same command plus `/L` and `/XX` (Copy) – nothing is touched:
  `robocopy "<A>" "<B>" /E /XX /L /R:1 /W:1 /NJH /NJS /NDL /NC /BYTES /FP /NP /UNILOG:<tmp>`
- **Never parse labels** (`New File`, `*EXTRA File`, `Newer`…): they are localized. Stdout is in the OEM code page (garbled Italian).
- `/UNILOG` is **UTF-16LE with BOM** – decode with `String::from_utf16`, never as UTF-8.
- Classify each preview line **by full path**:
  - path under B → extra → Deleted (Mirror only; with `/XX` in Copy it doesn't appear);
  - path under A and `B\<relative>` exists → Updated;
  - otherwise → New.
  Compare paths case-insensitively, normalise trailing backslashes, support UNC `\\server\share`.
- Windows errors appear in the log as `ERROR <n> (0x…)`: 2, 3 (not found), 5 (access denied), 53, 67 (network) → plain Italian messages (S6).
- Large trees (100k+ files): stream the log, keep counts, don't send every line to the UI.
- The app must not run elevated: mapped drives disappear under an elevated token.
