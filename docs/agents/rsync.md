# rsync – facts the engine relies on (macOS and Linux)

Source: spike of 2026-11-04 on the GitHub runners (`macos-latest` = macOS 26.6, `ubuntu-latest` = Ubuntu with rsync 3.4.1). CI tests on `macos-15` (macOS 15.8, openrsync) and `ubuntu-latest`; the "Which rsync" CI step prints `which rsync` / `rsync --version` in every run. Code: `src-tauri/src/rsync.rs`; fixtures in `src-tauri/tests/fixtures/rsync_preview_*.txt` are real captures. Don't contradict without a test proving otherwise.

## Which rsync
- Binary: `rsync` found on `PATH` (`/usr/bin/rsync` on both).
- **macOS ships openrsync, not GNU rsync**: `rsync --version` prints `openrsync: protocol version 29` / `rsync version 2.6.9 compatible`. Linux: GNU rsync 3.4.1, protocol 32 (Ubuntu 26.04; Ubuntu 24.04 ships 3.2.7).
- **macOS 14 and older ship the original samba rsync 2.6.9** (`rsync  version 2.6.9  protocol version 29`, samba.org copyright), not openrsync; macOS 15 and later ship openrsync (macOS 15.8 and 26.x checked on runners). The engine targets openrsync and CI runs on `macos-15`, so samba rsync on macOS 14 is **not tested** (the suite did pass there on 2026-11-05, but nothing guarantees it).
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
- A missing parent of B: GNU **11** (`mkdir … failed`), **openrsync exit 0 and creates every folder** above B – on macOS that would write on the internal disk when `/Volumes/Backup` is unplugged. Only B itself may be new: the preview answers like GNU's 11 without running rsync, and the run starts a stand-in (`sh -c 'exit 11'`) instead of rsync, so it is recorded as an Error run (cause: B not found) and nothing is created. A B that does not exist but whose parent does is a normal first run, never a cause.
- An unreadable file in A: **23** on both (GNU `… Permission denied (13)`; openrsync `open (2) in …: Permission denied` – the number in parentheses is **not** an errno there, so it is never parsed).
- A process killed by the app has no exit code (`ExitStatus::code()` is `None`): it reads as 128 + signal.
- Mapping to the S6 messages (`rsync::find_cause`): 3, 11, 23 → "non trovo la cartella A/B" (code 3) when that folder is missing now (B: only when its parent is missing too), "non ho il permesso" (code 5) when `read_dir` of A or B says permission denied; 10, 12, 30, 35 → "non raggiungibile" (code 53) only when a folder really is out of reach (metadata fails), otherwise the code is ambiguous and stays generic; any other failure → the generic message with "Apri il registro".
- rsync not installed: starting it fails with `NotFound`; the preview and the run show "Su questo computer manca rsync." (cause code 127 in the history), never the OS error. A failed start is recorded as an Error run.

## Run, progress, cancel
- **Is stdout live?** GNU rsync: yes, line-buffered even on a pipe (every line at 0.00 s). **openrsync: no** – with stdout on a pipe nothing arrives until it exits (all lines at 14.5 s on a 2 GB copy), and `--log-file` is not honoured by openrsync at all (no file). `script -q /dev/null rsync …` and `stdbuf -o0` both made it live on the runner, but `stdbuf` works through `DYLD_INSERT_LIBRARIES`, which SIP strips from system binaries, and `script` puts `^D` in front of the first line. So on macOS the run gives rsync a **pseudo-terminal** (`openpty`, close-on-exec on both ends) as stdout and stderr; elsewhere a plain pipe. The pty writes `\r\n`; the parser trims `\r`.
- Progress = count of file lines on the merged stdout (`>f…` and non-folder `*deleting`), exactly the lines the preview counted (total = new + updated + deleted). **A file's line is printed when rsync starts on it** (spike 3, three 1.5 GB files: `m2` appeared only after `m1` had finished, on both platforms), so the last line is the file in flight: the partial count of a stopped or aborted run is `>f` lines minus one. Exit 23 and 24 mean rsync ran to the end (files skipped or vanished): all `>f` lines count. A last line that is a `*deleting` is never a file in flight.
- The log of a run is written best effort (`LogTee`): a log that cannot be written is dropped and the output is still drained, or rsync would block on a full pipe.
- `*deleting` padding: GNU pads the word to the width of an itemize code (11) plus a space, openrsync does not; the padding is cut by width so names that start with spaces survive. The flavour is told by the width of the first itemize code in the log.
- Cancel = `Child::kill()` (SIGKILL) on the parent rsync; the receiver sees the pipe close and removes its temp file (`.name.XXXXXX`).
- Data folder: macOS `~/Library/Application Support/ColomboSync`, Linux `$XDG_DATA_HOME/ColomboSync` (absolute only) or `~/.local/share/ColomboSync`.
- Case: Linux paths are case-sensitive; macOS volumes usually are not, so the A/B path rules compare lower-cased there. Links are resolved first (deepest existing folder made canonical, the rest appended), so `/Volumes/Macintosh HD` (a link to `/`) cannot hide that B is inside A. The folder picker does not list links that resolve to `/`. Relative paths are refused ("Scrivi il percorso completo, che inizia con /.").
- MirrorGuard ignores `.DS_Store`, `.Spotlight-V100`, `.fseventsd`, `.Trashes`, `.TemporaryItems`, `lost+found` and `.Trash-*` when it asks whether A is empty (any case except on Linux).
