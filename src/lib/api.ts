// The only place that talks to Rust: Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import type { Counts, FolderListing, Preview, Run, SyncMode } from "./types";

export const previewSync = (source: string, destination: string, mode: SyncMode) =>
  invoke<Preview>("preview_sync", { source, destination, mode });

export const runSync = (source: string, destination: string, mode: SyncMode, counts: Counts) =>
  invoke<Run>("run_sync", { source, destination, mode, counts });

/** Drives when `path` is empty, otherwise the subfolders of `path`. */
export const browseFolders = (path: string) => invoke<FolderListing>("browse_folders", { path });

/** Past runs, newest first. */
export const listRuns = () => invoke<Run[]>("list_runs");

/** Text of a run's robocopy log; rejects when the file is gone. */
export const readRunLog = (path: string) => invoke<string>("read_run_log", { path });
