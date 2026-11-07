// The only place the wizard talks to Rust: Tauri commands. Each one rejects with an `Explanation`.
import { invoke } from "@tauri-apps/api/core";
import type { SyncMode } from "../shared/types";
import type { FolderListing, Preview } from "./types";

/** The preview, with the token that lets `runSync` confirm it. */
export const previewSync = (source: string, destination: string, mode: SyncMode) =>
  invoke<Preview>("preview_sync", { source, destination, mode });

/** Drives when `path` is empty, otherwise the subfolders of `path`. */
export const browseFolders = (path: string) => invoke<FolderListing>("browse_folders", { path });

/**
 * Makes the folder `name` inside `parent` as a B for the folder A `source` and resolves with its
 * path; rejects also when the new folder would be inside A.
 */
export const createFolder = (source: string, parent: string, name: string) =>
  invoke<string>("create_folder", { source, parent, name });

/** Rejects for a B that is A itself or inside A. */
export const checkDestination = (source: string, destination: string) =>
  invoke<void>("check_destination", { source, destination });
