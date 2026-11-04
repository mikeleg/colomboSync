# rsync – facts the engine relies on (macOS and Linux)

Source: spike of 2026-11-04 on the GitHub runners (`macos-latest` = macOS 26.6, `ubuntu-latest` = Ubuntu with rsync 3.4.1). Code: `src-tauri/src/rsync.rs`; fixtures in `src-tauri/tests/fixtures/rsync_preview_*.txt` are real captures. Don't contradict without a test proving otherwise.

## Which rsync
- Binary: `rsync` found on `PATH` (`/usr/bin/rsync` on both).
- **macOS ships openrsync, not GNU rsync**: `rsync --version` prints `openrsync: protocol version 29` / `rsync version 2.6.9 compatible`. Linux: GNU rsync 3.4.1, protocol 32.
- openrsync does **not** know `--mkpath`, `--protect-args`, and ignores `--modify-window` (a 1 s mtime difference is still reported with `--modify-window=2`). Don't use them.
- openrsync *does* know `-8`, `--itemize-changes`, `--out-format`, `--delete`, `--dry-run`, `--log-file`.

## Flags
- Copy: `rsync -rt -8 --itemize-changes [--dry-run] A/ B/`. Mirror: the same plus `--delete`. **Never `--delete` in Copy**, never `-a` (permissions, owners and links do not fit external exFAT/NTFS disks and would turn into exit 23).
- `-t` keeps mtimes, so the second run lists nothing (verified: a second `--delete` run prints only the symlink notice).
- `-8`: without it GNU rsync escapes non-ASCII names in a non-UTF-8 locale (`\#303\#250 gi\#303\#240.txt` with `LC_ALL=C`); with it names stay raw UTF-8. openrsync prints raw bytes anyway.
- Both paths get a trailing `/` (copy what is inside A into B).
- Symlinks are not copied with `-rt`: rsync prints `skipping non-regular file "link.txt"` on **stdout**, exit 0.

## Itemize output (stdout, one line per item: `<code> <name>`)
| Meaning | GNU 3.4.1 | openrsync |
|---|---|---|
| new file | `>f+++++++++ new.txt` | `>f+++++++ new.txt` |
| updated (size and time) | `>f.st...... updated.txt` | `>f.st.... updated.txt` |
| updated (time only) | `>f..t...... same.txt` | `>f..t.... same.txt` |
| extra file in B (`--delete`) | `*deleting   only-in-b.txt` | `*deleting only-in-b.txt` |
| extra folder in B | `*deleting   old-dir/` | `*deleting old-dir/` |
| folder (new B) | `cd+++++++++ sub/` | `cd+++++++ sub/` |

- The code is **11 characters on GNU and 9 on openrsync**; `*deleting` is padded with spaces on GNU only. Parse: first space-separated token = code, the rest is the name (trim for `*deleting`).
- Classify by the code: starts with `>f` = file copied (New if the code has `+`, else Updated); `*deleting` = extra file (skip names ending in `/`: those are folders); everything else (`cd…`, `cL…`, `skipping…`, `created directory …`, `sending incremental file list`) is not a file.
- Order of lines differs (GNU interleaves `*deleting` with files); never depend on it.
- **openrsync dry run into a B that does not exist** prints `>f.......` (no `+`) for every file: a dry run with a missing B is New for all, so the parser takes "B does not exist" as input. GNU prints `+++` correctly and adds `created directory <B>`.
- Itemize lines carry **no size**: the preview takes sizes from `A/<name>` on disk.
- stdout and stderr are merged into one pipe for a real run (that is also the run's log, UTF-8). A Preview keeps them apart and appends stderr after stdout.

## Exit codes (never read the English text)
- 0 success. Anything else is a failure for the app (23 "some files not transferred" and 24 "some files vanished" included: the user must hear the copy is incomplete).
- A missing A: **23** on both (GNU: `change_dir … failed: No such file or directory (2)`; openrsync: `(l)stat: No such file or directory`).
- A missing parent of B: GNU **11** (`mkdir … failed`), **openrsync exit 0 and creates every folder** above B – on macOS that would write on the internal disk when `/Volumes/Backup` is unplugged. The app refuses a B whose parent is missing (only B itself may be new) and treats it like GNU's 11.
- An unreadable file in A: **23** on both (GNU `… Permission denied (13)`; openrsync `open (2) in …: Permission denied` – the number in parentheses is **not** an errno there, so it is never parsed).
- A process killed by the app has no exit code (`ExitStatus::code()` is `None`): it reads as 128 + signal.
- Mapping to the S6 messages (`rsync::find_cause`): 10, 12, 30, 35 → "non raggiungibile" (code 53); 3, 11, 23 → "non trovo la cartella A/B" (code 3) when that folder does not exist now, "non ho il permesso" (code 5) when `read_dir` of A or B says permission denied; any other failure → the generic message with "Apri il registro".

## Run, progress, cancel
- Progress = count of file lines on the merged stdout (`>f…` and non-folder `*deleting`), exactly the lines the preview counted (total = new + updated + deleted). A file is listed when rsync **starts** on it, so the last line is the file in flight: the partial count of an Error/Cancelled run is `>f` lines minus one.
- Cancel = `Child::kill()` (SIGKILL) on the parent rsync; the receiver sees the pipe close and removes its temp file (`.name.XXXXXX`).
- Data folder: macOS `~/Library/Application Support/ColomboSync`, Linux `$XDG_DATA_HOME/ColomboSync` (absolute only) or `~/.local/share/ColomboSync`.
- Case: Linux paths are case-sensitive; macOS volumes usually are not, so the A/B path rules compare lower-cased there.
