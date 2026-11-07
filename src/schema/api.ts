// The schema panel talks to Rust through this Tauri command.
import { invoke } from "@tauri-apps/api/core";
import type { Place } from "../shared/types";

/** What kind of place the folder `path` is: a share or a drive, and the label to show for it. */
export const describePlace = (path: string) => invoke<Place>("describe_place", { path });
