import type { Counts } from "./types";

/** What SchemaView draws. Whoever owns the current screen builds it and hands it to App. */
export interface SchemaModel {
  source: string;
  destination: string;
  /** null shows "—" instead of numbers. */
  counts: Counts | null;
  caption: string;
  /** Mirror mode: B and the arrow turn red and an "Eliminati da B" counter appears. */
  mirror: boolean;
  /** Draw the caption as a warning (red). */
  warnCaption: boolean;
}

export const emptySchema: SchemaModel = {
  source: "",
  destination: "",
  counts: null,
  caption: "Crea una sincronizzazione per vedere lo schema.",
  mirror: false,
  warnCaption: false,
};
