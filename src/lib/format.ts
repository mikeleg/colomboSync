export const formatNumber = (n: number) => n.toLocaleString("it-IT");

export function formatSize(bytes: number): string {
  if (bytes >= 1e9) return (bytes / 1e9).toFixed(1).replace(".", ",") + " GB";
  return Math.max(1, Math.round(bytes / 1e6)) + " MB";
}

export function formatDuration(ms: number): string {
  const minutes = Math.round(ms / 60000);
  return minutes < 1 ? "< 1 min" : `${minutes} min`;
}
