// The home and the wizard's save step talk to Rust through these Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import type { Sync, SyncMode } from "../shared/types";
import type { ListedSyncs } from "./types";

/** Saved syncs, newest first, each with how its latest Run ended. */
export const listSyncs = () => invoke<ListedSyncs>("list_syncs");

export const saveSync = (name: string, source: string, destination: string, mode: SyncMode) =>
  invoke<Sync>("save_sync", { name, source, destination, mode });

/** "Copia di <last folder of A>". */
export const defaultSyncName = (source: string) => invoke<string>("default_sync_name", { source });
