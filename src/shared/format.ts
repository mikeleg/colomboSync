import type { Outcome, SyncMode } from "./types";

export const formatNumber = (n: number) => n.toLocaleString("it-IT");

export function formatSize(bytes: number): string {
  if (bytes >= 1e9) return (bytes / 1e9).toFixed(1).replace(".", ",") + " GB";
  return Math.max(1, Math.round(bytes / 1e6)) + " MB";
}

/** Chip class (from theme.css) and Italian label, as used by the Storico rows and its legend. */
export const modeChip: Record<SyncMode, { cls: string; label: string }> = {
  Copy: { cls: "copy", label: "Copia" },
  Mirror: { cls: "mirror", label: "Specchio" },
};

export const outcomeChip: Record<Outcome, { cls: string; label: string }> = {
  Ok: { cls: "ok", label: "OK" },
  Error: { cls: "err", label: "Errore" },
  Cancelled: { cls: "ann", label: "Annullato" },
};
