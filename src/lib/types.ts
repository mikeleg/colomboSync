// Shapes returned by the Rust commands (src-tauri/src/preview_parser.rs, history.rs).

export type SyncMode = "Copy" | "Mirror";

export type Outcome = "Ok" | "Error" | "Cancelled";

export interface FileGroup {
  count: number;
  /** First files only (relative paths); the rest is only counted. */
  files: string[];
}

export interface Preview {
  new: FileGroup;
  updated: FileGroup;
  deleted: FileGroup;
  bytes: number;
  /** MirrorGuard: why a mirror run must not start (Italian text from Rust); null when it may. */
  blocked: string | null;
}

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

/** One sync found in the history (src-tauri/src/history_summary.rs). */
export interface SyncStatus {
  id: number;
  name: string;
  /** Start time of its latest successful run. */
  lastSuccess: string | null;
  /** Italian sentence from Rust, set when its latest run ended in Errore. */
  warning: string | null;
}

/** What `history_summary` returns: totals over the whole history and the state of each sync. */
export interface HistorySummary {
  runs: number;
  ok: number;
  failed: number;
  cancelled: number;
  /** New + updated files, plus those copied by failed or stopped runs. */
  filesCopied: number;
  filesDeleted: number;
  syncs: SyncStatus[];
}

/** Why something failed, as Rust words it: what happened, and what to do about it. */
export interface Explanation {
  title: string;
  advice: string;
}

/** What `run_sync` returns: the stored Run, plus the explanation when it ended in Errore. */
export type RunReport = Run & { explanation: Explanation | null };

/** The `run-progress` event while a Run is going. */
export interface RunProgress {
  syncName: string;
  done: number;
  total: number;
  elapsedMs: number;
  currentFile: string;
}

interface Folder {
  name: string;
  path: string;
}

/** Drives (path "") or the subfolders of a path; `parent` is "" when the level above is the list of drives. */
export interface FolderListing {
  parent: string;
  folders: Folder[];
  /** Plain Italian message when the folder could not be opened. */
  error: string | null;
}

/** A saved A→B pair (src-tauri/src/sync_store.rs). */
export interface Sync {
  id: number;
  name: string;
  source: string;
  destination: string;
  mode: SyncMode;
}

/** A saved sync as the home lists it, with how its latest Run ended (null: never run). */
export interface SyncSummary extends Sync {
  lastRun: { startedAt: string; outcome: Outcome; note: string | null } | null;
}

/** What the home lists; `setAside`: a damaged syncs.json was renamed aside and the list starts empty. */
export interface ListedSyncs {
  syncs: SyncSummary[];
  setAside: boolean;
}
