// Shapes returned by the home's Rust commands (src-tauri/src/shared/sync_store.rs, src-tauri/src/history/commands.rs).

import type { Outcome, Sync } from "../shared/types";

/** A saved sync as the home lists it, with how its latest Run ended (null: never run). */
export interface SyncSummary extends Sync {
  lastRun: { startedAt: string; outcome: Outcome; note: string | null } | null;
}

/** What the home lists; `setAside`: a damaged syncs.json was renamed aside and the list starts empty. */
export interface ListedSyncs {
  syncs: SyncSummary[];
  setAside: boolean;
}
