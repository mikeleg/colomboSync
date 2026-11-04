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
  /** MirrorGuard: a mirror run must not start (A missing or empty, or too many files to delete). */
  blocked: boolean;
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
