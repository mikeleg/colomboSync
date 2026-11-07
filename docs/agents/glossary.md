# Glossary (ubiquitous language)

Use these names in Rust and TypeScript. UI shows the Italian column.

| Code | UI (it) | Meaning |
|---|---|---|
| `Sync` / `SavedSync` | sincronizzazione | Saved A→B pair: `name`, `source` (A), `destination` (B), `mode`. `SyncStore` is `syncs.json`. |
| `source` / A | Da dove / A | Folder copied from. Local path, mapped drive or UNC on Windows; `/`-rooted path on macOS/Linux. |
| `destination` / B | Dove / B | Folder copied to. Never inside or equal to A, and A is never inside B. |
| `Engine` | – | The program that copies: **robocopy** on Windows, **rsync** on macOS (openrsync) and Linux (GNU rsync). Picked by `cfg` in `engine/mod.rs`; nothing outside `engine/` (`mod.rs`, `robocopy.rs`, `rsync.rs`) knows which one runs. |
| `SyncMode::Copy` | Copia | Default. Never deletes in B (robocopy `/E`, rsync `-rt` without `--delete`). |
| `SyncMode::Mirror` | Specchio | Makes B identical to A – deletes extras in B (robocopy `/MIR`, rsync `--delete`). Needs explicit consent per run. |
| `Preview` | Anteprima | Dry run (robocopy `/L`, rsync `--dry-run`): lists/counts of `new`, `updated`, `deleted`. Mandatory before every `Run`. |
| `PreviewMemo` | – | The last Preview shown (A, B, mode, counts), kept in Rust with its opaque `previewToken` (the one `preview_sync` sends to the UI). `run_sync(sync, previewToken, consent)` takes its counts from it, never from the UI, and is refused unless the token is the last Preview's, for the same A, B and mode; a Mirror that deletes also needs the `consent`. A blocked Preview has no token. Each Preview confirms one Run. |
| `Run` | copia / esecuzione | One execution of a `Sync`. Outcome `Ok` / `Error` / `Cancelled` (OK / Errore / Annullato). |
| `ActiveRun` | – | Managed state holding the engine process of the Run in flight, so the stop button and the window close can kill it. At most one Run at a time. |
| `partial` | N file copiati (parziale) | `Run.partial`: files copied before an `Error` or `Cancelled` Run stopped, a single total with no new/updated split. The file in flight is not counted. `Ok` Runs keep the Preview counts instead. |
| `History` | Storico | Append-only `history.jsonl`, one `Run` per line, newest shown first. Malformed lines are skipped. |
| `Cause` | – | `{code, folder}` – what went wrong (a mapped code: 2, 3, 5, 53, 67, 127 = engine missing; `folder` is A or B). It is what `history.jsonl` stores, mapped from the engine's exit code or error code, never from its text. |
| `Explanation` | – | `{title, advice}` – the plain-Italian sentence for the user (what happened, what to do), made in Rust from a `Cause` and the `Moment` (preview or run) when it is shown. The UI never composes messages. It is the only error a command returns (`Result<T, Explanation>`, built in `shared/error_messages.rs`); Svelte renders `title` and `advice`. |
| `MirrorGuard` | – | Blocks a mirror run when A is missing or empty, or when deleted > new + updated. Reads the counts Rust remembered (`PreviewMemo`), not what the UI sends. |
| `SchemaView` | schema | The A → B diagram with counters, in the side panel of home, wizard, run and result (Storico shows the history legend instead). |
