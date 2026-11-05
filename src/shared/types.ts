// Shapes returned by the Rust commands (src-tauri/src/history.rs, sync_store.rs, run_sync). Used by two or more slices.

export type SyncMode = "Copy" | "Mirror";

export type Outcome = "Ok" | "Error" | "Cancelled";

export interface Counts {
  new: number;
  updated: number;
  deleted: number;
}

export interface Run {
  /** The saved sync it belongs to; missing on lines written before syncs had ids. */
  syncId?: number;
  syncName: string;
  source: string;
  destination: string;
  mode: SyncMode;
  new: number;
  updated: number;
  deleted: number;
  durationMs: number;
  outcome: Outcome;
  startedAt: string;
  logPath: string;
  /** Files copied before an Error or Cancelled run stopped. */
  partial?: number;
}

/** Why something failed, as Rust words it: what happened, and what to do about it. */
export interface Explanation {
  title: string;
  advice: string;
}

/** What `run_sync` returns: the stored Run, plus the explanation when it ended in Errore. */
export type RunReport = Run & { explanation: Explanation | null };

/** A saved A→B pair (src-tauri/src/sync_store.rs). */
export interface Sync {
  id: number;
  name: string;
  source: string;
  destination: string;
  mode: SyncMode;
}
