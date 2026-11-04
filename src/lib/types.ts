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
}

export interface Counts {
  new: number;
  updated: number;
  deleted: number;
}

export interface Run {
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
  error?: string;
  /** Files copied before an Error or Cancelled run stopped. */
  partial?: number;
}

/** The `run-progress` event while a Run is going. */
export interface RunProgress {
  syncName: string;
  done: number;
  total: number;
  elapsedMs: number;
  currentFile: string;
}
