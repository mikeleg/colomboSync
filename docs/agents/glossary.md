# Glossary (ubiquitous language)

Use these names in Rust and TypeScript. UI shows the Italian column.

| Code | UI (it) | Meaning |
|---|---|---|
| `Sync` | sincronizzazione | Saved A→B pair: `name`, `source` (A), `destination` (B), `mode`. |
| `source` / A | Da dove / A | Folder copied from. Local path, mapped drive or UNC. |
| `destination` / B | Dove / B | Folder copied to. Never inside or equal to A. |
| `SyncMode::Copy` | Copia | Default. `/E`. Never deletes in B. |
| `SyncMode::Mirror` | Specchio | `/MIR`. Makes B identical to A – deletes extras in B. Needs explicit consent per run. |
| `Preview` | Anteprima | Result of `/L`: lists/counts of `new`, `updated`, `deleted`. Mandatory before every `Run`. |
| `Run` | copia / esecuzione | One execution of a `Sync`. Outcome `Ok` / `Error` / `Cancelled` (OK / Errore / Annullato). |
| `History` | Storico | Append-only `history.jsonl`, one `Run` per line, newest shown first. |
| `SyncStore` | Le tue sincronizzazioni | `syncs.json`. |
| `MirrorGuard` | – | Blocks a mirror run when A is missing or empty, or when deleted > new + updated. |
| `SchemaView` | schema | The A → B diagram with counters, visible on home and in the wizard/run/result (Storico shows the history legend instead). |
