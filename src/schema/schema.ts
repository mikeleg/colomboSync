import type { Counts, Outcome, Place } from "../shared/types";

/** What SchemaView draws. Whoever owns the current screen builds it and hands it to App. */
export interface SchemaModel {
  source: string;
  destination: string;
  /** What kind of place each path is, when the owner already knows; otherwise SchemaView asks Rust. */
  sourcePlace?: Place;
  destinationPlace?: Place;
  /** null shows "—" instead of numbers. */
  counts: Counts | null;
  caption: string;
  /** Mirror mode: B and the arrow turn red and an "Eliminati da B" counter appears. */
  mirror: boolean;
  /** Draw the caption as a warning (red). */
  warnCaption: boolean;
  /** A run is going: the arrow is animated. */
  running?: boolean;
  /** How the run ended: Ok and Error each get one pulse on B. */
  outcome?: Outcome;
}

export const emptySchema: SchemaModel = {
  source: "",
  destination: "",
  counts: null,
  caption: "Crea una sincronizzazione per vedere lo schema.",
  mirror: false,
  warnCaption: false,
};
