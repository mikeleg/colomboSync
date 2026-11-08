import type { Outcome, SyncMode } from "./types";

const numbers = new Intl.NumberFormat("it-IT");
const gigabytes = new Intl.NumberFormat("it-IT", {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1,
  useGrouping: false,
});

export const formatNumber = (n: number) => numbers.format(n);

export function formatSize(bytes: number): string {
  if (bytes >= 1e9) return `${gigabytes.format(bytes / 1e9)} GB`;
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
