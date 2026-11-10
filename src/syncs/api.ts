// The home talks to Rust through these Tauri commands. Those that can fail reject with an `Explanation`.
import { invoke } from "@tauri-apps/api/core";
import type { ListedSyncs } from "./types";

/** Saved syncs, newest first, each with how its latest Run ended. */
export const listSyncs = () => invoke<ListedSyncs>("list_syncs");
