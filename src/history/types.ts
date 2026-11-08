// Shapes returned by `list_runs` (src-tauri/src/history/commands.rs, history/summary.rs).

import type { Explanation, RunReport } from "../shared/types";

/** One sync found in the history (src-tauri/src/history/summary.rs). */
export interface SyncStatus {
  id: number;
  name: string;
  /** Italian sentence from Rust: when its latest successful run started, or that there is none. */
  lastSuccessText: string;
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
  runs: RunReport[];
  summary: HistorySummary;
}

/** The Storico screen while `list_runs` is asked, after it failed (Rust's explanation), and once it answered. */
export type HistoryState =
  { status: "loading" } | { status: "failed"; error: Explanation } | ({ status: "ready" } & RunList);
