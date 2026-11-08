import type { RunReport } from "../shared/types";

/** A finished run as Rust sends it, texts included: the screens show them as they are (see the tests of shared/copy_text.rs). */
export const run = (over: Partial<RunReport> = {}): RunReport => ({
  syncName: "Copia di Foto",
  source: String.raw`D:\Foto`,
  destination: String.raw`E:\Backup`,
  mode: "Copy",
  new: 12,
  updated: 1,
  deleted: 0,
  durationMs: 61_000,
  outcome: "Ok",
  startedAt: "2026-11-02T21:30:00+01:00",
  logPath: String.raw`C:\L\logs\a.log`,
  explanation: null,
  title: "Fatto: 12 file copiati, 1 aggiornato in 1 min",
  caption: "Fatto.",
  resultNote: null,
  partialText: null,
  when: "02/11/2026 21:30",
  durationText: "1 min",
  ...over,
});
