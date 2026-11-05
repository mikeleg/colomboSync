// Shapes returned by `list_runs` (src-tauri/src/history.rs, history_summary.rs).

import type { Run } from "../shared/types";

/** One sync found in the history (src-tauri/src/history_summary.rs). */
export interface SyncStatus {
  id: number;
  name: string;
  /** Start time of its latest successful run. */
  lastSuccess: string | null;
  /** Italian sentence from Rust, set when its latest run ended in Errore. */
  warning: string | null;
}

/** Totals over the whole history and the state of each sync. */
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

/** What `list_runs` returns for the Storico screen. */
export interface RunList {
  /** Newest first. */
  runs: Run[];
  summary: HistorySummary;
}
