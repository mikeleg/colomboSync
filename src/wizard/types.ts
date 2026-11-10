// Shapes returned by the wizard's Rust commands (src-tauri/src/engine/preview_parser.rs, wizard/commands.rs, wizard/folder_browser.rs).

/** The wizard's steps (`STEPS` in Wizard.svelte). */
export type StepId = "source" | "destination" | "name" | "options" | "preview";

/** The steps Rust checks on Avanti (`check_step`): Opzioni never refuses, Anteprima has no Avanti. */
export type CheckedStepId = "source" | "destination" | "name";

export interface FileGroup {
  count: number;
  /** First files only (relative paths); the rest is only counted. */
  files: string[];
}

/** A piece of a sentence Rust made; the counts are `bold`. */
export interface Segment {
  text: string;
  bold: boolean;
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
  /** The schema panel's caption: the `blocked` reason, or what there is to do ("B è già aggiornato."). */
  caption: string;
  /** "Ecco cosa verrà fatto: 148 file nuovi, 1 da aggiornare.", in pieces: the counts are bold. */
  summary: Segment[];
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

/** The `run-progress` event while a Run is going. */
export interface RunProgress {
  syncName: string;
  done: number;
  currentFile: string;
  /** Time left, in words, as Rust says it. */
  eta: string;
}
