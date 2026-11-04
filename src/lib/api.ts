// The only place that talks to Rust: Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import type { Counts, FolderListing, Preview, Run, SyncMode } from "./types";

export const previewSync = (source: string, destination: string, mode: SyncMode) =>
  invoke<Preview>("preview_sync", { source, destination, mode });

export const runSync = (source: string, destination: string, mode: SyncMode, counts: Counts) =>
  invoke<Run>("run_sync", { source, destination, mode, counts });

/** Drives when `path` is empty, otherwise the subfolders of `path`. */
export const browseFolders = (path: string) => invoke<FolderListing>("browse_folders", { path });

/** Rejects (with the Italian message to show) a B that is A itself or inside A. */
export const checkDestination = (source: string, destination: string) =>
  invoke<void>("check_destination", { source, destination });
