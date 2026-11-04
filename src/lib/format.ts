export const formatNumber = (n: number) => n.toLocaleString("it-IT");

export function formatSize(bytes: number): string {
  if (bytes >= 1e9) return (bytes / 1e9).toFixed(1).replace(".", ",") + " GB";
  return Math.max(1, Math.round(bytes / 1e6)) + " MB";
}

export function formatDuration(ms: number): string {
  const minutes = Math.round(ms / 60000);
  return minutes < 1 ? "< 1 min" : `${minutes} min`;
}

/** "2026-11-02T21:30:00+01:00" -> "02/11/2026 21:30": the local time as it was stored. */
export function formatWhen(startedAt: string): string {
  const [, year, month, day, time] = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}:\d{2})/.exec(startedAt) ?? [];
  return `${day}/${month}/${year} ${time}`;
}
