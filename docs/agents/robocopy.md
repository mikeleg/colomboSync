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

## Real run and live progress (S5)
- Run = `robocopy "<A>" "<B>" /E /XX /R:1 /W:1 /NJH /NJS /NDL /NC /BYTES /FP /NP /TEE /UNILOG:<log>` (Mirror: `/MIR`, no `/XX`). `/XX` keeps extras in B out of the output, so every file line is a file of A. `/TEE` streams the same lines to stdout, which the app reads through a pipe; `/UNILOG` stays the history log.
- Stdout is **not** UTF-16: it is the console OEM code page (also with `/UNICODE`), decoded with `MultiByteToWideChar(CP_OEMCP)`. It starts with a ` Log File : <path>` header (localized, even with `/NJH`) and blank lines.
- Count progress without reading labels: a **file line is the only line that starts with a tab**. `ERROR` lines start with a date.
- `Child::kill()` is `TerminateProcess(…, 1)`: the exit code of a stopped run is **1**, which reads as success. The Run is `Cancelled` because the app asked for the stop, never because of the exit code.
- A partial count (Error/Cancelled) counts the file lines of the run's own `/UNILOG` under A.  The last file line is the file in flight (maybe cut short), so it is not counted. Never classify them like a preview: after a copy every file exists in B.

## Error lines in the log (Registro)
- The log window marks a line as an error when it holds `<n> (0x<hex>)` (any `n`, any label before it) and is not a file line (file lines start with a tab). The sentence Windows writes on the next line is not marked.
