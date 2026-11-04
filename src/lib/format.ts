export const formatNumber = (n: number) => n.toLocaleString("it-IT");

export const isNetworkPath = (path: string) => path.startsWith("\\\\");

/** "02/11/2026 21:30" from an RFC 3339 start time, as recorded (local time, no conversion). */
export function formatWhen(startedAt: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(startedAt);
  return m ? `${m[3]}/${m[2]}/${m[1]} ${m[4]}:${m[5]}` : startedAt;
}

export function formatSize(bytes: number): string {
  if (bytes >= 1e9) return (bytes / 1e9).toFixed(1).replace(".", ",") + " GB";
  return Math.max(1, Math.round(bytes / 1e6)) + " MB";
}

export function formatDuration(ms: number): string {
  const minutes = Math.round(ms / 60000);
  return minutes < 1 ? "< 1 min" : `${minutes} min`;
}
