import type { Counts } from "./types";

/** What SchemaView draws. Whoever owns the current screen builds it and hands it to App. */
export interface SchemaModel {
  source: string;
  destination: string;
  /** null shows "—" instead of numbers. */
  counts: Counts | null;
  caption: string;
  /** A run is going: the arrow is animated. */
  running?: boolean;
}

export const emptySchema: SchemaModel = {
  source: "",
  destination: "",
  counts: null,
  caption: "Crea una sincronizzazione per vedere lo schema.",
};
