// Shapes returned by the Rust commands (src-tauri/src/history/commands.rs, shared/sync_store.rs, run/commands.rs). Used by two or more slices.

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

/**
 * What `run_sync` and `list_runs` return: the stored Run, plus what Rust says about it (every text
 * is finished: the UI only shows it).
 */
export type RunReport = Run & {
  /** Set when it ended in Errore. */
  explanation: Explanation | null;
  /** Heading of the result: what was done, or why it was not. */
  title: string;
  /** The schema panel's one word for how it ended ("Fatto."). */
  caption: string;
  /** What the result says about the files copied before an Error or Cancelled run stopped. */
  resultNote: string | null;
  /** "40 file copiati (parziale)"; set with `partial`. */
  partialText: string | null;
  /** When it started, "02/11/2026 21:30". */
  when: string;
  /** How long it took, "3 min". */
  durationText: string;
};

/** A saved A→B pair (src-tauri/src/shared/domain.rs, SavedSync). */
export interface Sync {
  id: number;
  name: string;
  source: string;
  destination: string;
  mode: SyncMode;
}

/** What `read_run_log` returns (src-tauri/src/history/mod.rs, RunLog): the text, and which of its lines (from 0) are engine errors. */
export interface RunLog {
  text: string;
  errorLines: number[];
}
