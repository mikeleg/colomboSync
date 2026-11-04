// The only place that talks to Rust: Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import type { Counts, FolderListing, Preview, Run, Sync, SyncMode, SyncSummary } from "./types";

export const previewSync = (source: string, destination: string, mode: SyncMode) =>
  invoke<Preview>("preview_sync", { source, destination, mode });

export const runSync = (source: string, destination: string, mode: SyncMode, counts: Counts, syncName: string) =>
  invoke<Run>("run_sync", { source, destination, mode, counts, syncName });

/** Saved syncs, newest first, each with how its latest Run ended. */
export const listSyncs = () => invoke<SyncSummary[]>("list_syncs");

export const saveSync = (name: string, source: string, destination: string, mode: SyncMode) =>
  invoke<Sync>("save_sync", { name, source, destination, mode });

/** "Copia di <last folder of A>". */
export const defaultSyncName = (source: string) => invoke<string>("default_sync_name", { source });

/** Drives when `path` is empty, otherwise the subfolders of `path`. */
export const browseFolders = (path: string) => invoke<FolderListing>("browse_folders", { path });
