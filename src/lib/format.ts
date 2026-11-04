export const formatNumber = (n: number) => n.toLocaleString("it-IT");

export function formatSize(bytes: number): string {
  if (bytes >= 1e9) return (bytes / 1e9).toFixed(1).replace(".", ",") + " GB";
  return Math.max(1, Math.round(bytes / 1e6)) + " MB";
}

export function formatDuration(ms: number): string {
  const minutes = Math.round(ms / 60000);
  return minutes < 1 ? "< 1 min" : `${minutes} min`;
}

/** Time left, from the files done so far in `elapsedMs`. */
export function etaText(done: number, total: number, elapsedMs: number): string {
  if (elapsedMs < 1000 || done === 0) return "Calcolo il tempo che manca…";
  const seconds = Math.ceil((total - done) / (done / (elapsedMs / 1000)));
  return seconds <= 1 ? "Quasi finito." : `Mancano circa ${formatNumber(seconds)} secondi.`;
}

export const partialText = (n: number) => `${formatNumber(n)} ${n === 1 ? "file copiato" : "file copiati"} (parziale)`;
