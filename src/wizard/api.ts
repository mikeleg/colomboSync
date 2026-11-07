// The only place the wizard talks to Rust: Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import type { Explanation, SyncMode } from "../shared/types";
import type { FolderListing, Preview } from "./types";

/** Anything that is not an `Explanation` (the call itself failed) becomes a title. */
const asExplanation = (e: unknown): Explanation =>
  typeof e === "object" && e !== null && "title" in e ? (e as Explanation) : { title: String(e), advice: "" };

/** Rejects with an `Explanation`. */
export const previewSync = (source: string, destination: string, mode: SyncMode) =>
  invoke<Preview>("preview_sync", { source, destination, mode }).catch((e: unknown) => {
    throw asExplanation(e);
  });

/** Drives when `path` is empty, otherwise the subfolders of `path`. */
export const browseFolders = (path: string) => invoke<FolderListing>("browse_folders", { path });

/**
 * Makes the folder `name` inside `parent` as a B for the folder A `source` and resolves with its
 * path; rejects with an `Explanation` (also when the new folder would be inside A).
 */
export const createFolder = (source: string, parent: string, name: string) =>
  invoke<string>("create_folder", { source, parent, name }).catch((e: unknown) => {
    throw asExplanation(e);
  });

/** Rejects (with the Italian message to show) a B that is A itself or inside A. */
export const checkDestination = (source: string, destination: string) =>
  invoke<void>("check_destination", { source, destination });
