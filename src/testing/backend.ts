import { invoke } from "@tauri-apps/api/core";
import { vi } from "vitest";
import { run } from "./fixtures";
import type { RunReport } from "../shared/types";
import type { SyncSummary } from "../syncs/types";
import type { Preview } from "../wizard/types";

export const invokeMock = vi.mocked(invoke);

export const preview = (n: number, u: number): Preview => ({
  new: { count: n, files: ["sub\\a.jpg"] },
  updated: { count: u, files: u ? ["b.pdf"] : [] },
  deleted: { count: 0, files: [] },
  bytes: 3_000_000,
  blocked: null,
  previewToken: "token-1",
  // The sentences Rust makes for these counts in Copy mode (see shared/copy_text.rs).
  caption: n + u === 0 ? "B è già aggiornato." : `Da fare: copiare ${n} file nuovi, aggiornarne ${u}.`,
  summary: [
    { text: "Ecco cosa verrà fatto: ", bold: false },
    { text: String(n), bold: true },
    { text: " file nuovi, ", bold: false },
    { text: String(u), bold: true },
    { text: " da aggiornare.", bold: false },
  ],
});

// A run as Rust sends it, texts included: the screens show them as they are.
export const doneRun = run({
  new: 2,
  durationMs: 2_000,
  startedAt: "2026-11-03T12:00:00+01:00",
  logPath: "C:\\L\\logs\\x.log",
  title: "Fatto: 2 file copiati, 1 aggiornato in < 1 min",
  when: "03/11/2026 12:00",
  durationText: "< 1 min",
});

// What Rust sends (error_messages): the sentence that says what happened, and what to do.
export const unreachable = {
  title: String.raw`Non completato: la cartella di rete \\NAS non è raggiungibile.`,
  advice: "Controlla la connessione e riprova.",
};
export const failedRun: RunReport = {
  ...doneRun,
  outcome: "Error",
  explanation: unreachable,
  partial: 0,
  title: unreachable.title,
  caption: "Non completato.",
  resultNote: "Nessun file è stato copiato.",
  partialText: "0 file copiati (parziale)",
};

export const savedSync: SyncSummary = {
  id: 7,
  name: "Foto di Anna",
  source: "D:\\Foto",
  destination: "E:\\Backup",
  mode: "Copy",
  sourcePlace: { isNetwork: false, label: "Disco D:" },
  destinationPlace: { isNetwork: false, label: "Disco E:" },
  lastRun: { when: "02/11/2026 21:30", outcome: "Ok", note: null },
};

type Args = Record<string, unknown> | undefined;
type Handlers = Partial<
  Record<
    | "preview_sync"
    | "run_sync"
    | "list_syncs"
    | "save_sync"
    | "default_sync_name"
    | "check_step"
    | "list_runs"
    | "is_elevated",
    (args: Args) => unknown
  >
>;

/** Answers the Rust commands the app uses; by default the computer has no folders and no saved syncs. */
export function backend(handlers: Handlers = {}) {
  const defaults: Record<string, (args: Args) => unknown> = {
    browse_folders: () => ({ parent: "", folders: [], error: null }),
    // What `describe_place` says for the paths these tests use (see shared/copy_text.rs).
    describe_place: (args) => {
      const labels: Record<string, string> = { "D:\\Foto": "Disco D:", "E:\\Backup": "Disco E:" };
      return { isNetwork: false, label: labels[(args as { path: string }).path] ?? "Cartella" };
    },
    list_syncs: () => ({ syncs: [], setAside: false }),
    default_sync_name: () => "Copia di Foto",
    // Rust decides what `check_step` refuses (wizard/commands.rs): a test that needs a refusal throws it itself.
    check_step: () => {},
    is_elevated: () => false,
    save_sync: (args) => ({ id: 7, ...args }),
  };
  invokeMock.mockImplementation(async (cmd, args) => {
    const handler = (handlers as Record<string, (args: Args) => unknown>)[cmd] ?? defaults[cmd];
    if (!handler) throw new Error(`unexpected command ${cmd}`);
    return handler(args as Args);
  });
}

export const calls = (cmd: string) => invokeMock.mock.calls.filter(([c]) => c === cmd);
