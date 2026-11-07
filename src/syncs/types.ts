// Shapes returned by the home's Rust commands (src-tauri/src/shared/sync_store.rs, src-tauri/src/history/commands.rs).

import type { Outcome, Place, Sync } from "../shared/types";

/** A saved sync as the home lists it, with what kind of places A and B are and how its latest Run ended (null: never run). */
export interface SyncSummary extends Sync {
  sourcePlace: Place;
  destinationPlace: Place;
  /** `when`: when it started, "02/11/2026 21:30", as Rust words it. */
  lastRun: { when: string; outcome: Outcome; note: string | null } | null;
}

/** What the home lists; `setAside`: a damaged syncs.json was renamed aside and the list starts empty. */
export interface ListedSyncs {
  syncs: SyncSummary[];
  setAside: boolean;
}
