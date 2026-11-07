// Shapes returned by the wizard's Rust commands (src-tauri/src/preview_parser.rs, folder browsing).

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
  /** What `runSync` must bring back to start a run from this preview; null when the preview is blocked. */
  previewToken: string | null;
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
