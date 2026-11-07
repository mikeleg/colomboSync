import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, expect, test, vi } from "vitest";
import type { RunReport } from "../shared/types";
import StepResult from "./StepResult.svelte";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);
beforeEach(() => invokeMock.mockReset());

const handlers = { onhome: () => {}, onretry: () => {}, onhistory: () => {} };

const run = (over: Partial<RunReport> = {}): RunReport => ({
  syncName: "Copia di Foto",
  source: "D:\\Foto",
  destination: "E:\\Backup",
  mode: "Copy",
  new: 12,
  updated: 3,
  deleted: 0,
  durationMs: 4_000,
  outcome: "Ok",
  startedAt: "2026-11-03T12:00:00+01:00",
  logPath: "C:\\L\\x.log",
  explanation: null,
  // What Rust sends for this run (see the tests of shared/copy_text.rs): the screen shows it as it is.
  title: "Fatto: 12 file copiati, 3 aggiornati in < 1 min",
  caption: "Fatto.",
  resultNote: null,
  partialText: null,
  when: "03/11/2026 12:00",
  durationText: "< 1 min",
  ...over,
});

// What Rust sends for a failure it has no cause for.
const generic = {
  title: "Non completato: la copia si è fermata per un problema.",
  advice: "Apri il registro per vedere i dettagli.",
};
const failed = (over: Partial<RunReport> = {}) =>
  run({ outcome: "Error", explanation: generic, title: generic.title, caption: "Non completato.", ...over });
const NOTHING_COPIED = "Nessun file è stato copiato.";

test("success: Fatto with copied and updated counts and the duration", () => {
  render(StepResult, { run: run(), failure: null, ...handlers });

  expect(screen.getByRole("heading").textContent).toBe("Fatto: 12 file copiati, 3 aggiornati in < 1 min");
  expect(screen.getByRole("button", { name: "Torna all’elenco" })).toBeTruthy();
});

test("the heading is the title Rust made, as it is", () => {
  const title = "Fatto: 1 file copiato, 1 aggiornato, 1 cancellato da B in 2 min";
  render(StepResult, { run: run({ title }), failure: null, ...handlers });

  expect(screen.getByRole("heading").textContent).toBe(title);
});

test("error outcome without a known cause: the generic title and where to look", () => {
  render(StepResult, {
    run: failed(),
    failure: null,
    ...handlers,
  });

  expect(screen.getByRole("heading").textContent).toBe(generic.title);
  expect(screen.getByText(generic.advice)).toBeTruthy();
  expect(screen.queryByText(/^Fatto/)).toBeNull();
});

test("a command failure before any Run exists shows Rust's title and advice", () => {
  const failure = {
    title: "Non ho avviato la copia: manca l’anteprima.",
    advice: "Prima controlla le due cartelle con l’anteprima, poi avvia.",
  };
  render(StepResult, { run: null, failure, ...handlers });

  expect(screen.getByRole("heading").textContent).toBe(failure.title);
  expect(screen.getByText(failure.advice)).toBeTruthy();
});

test("cancelled: Annullato with what stays in B, and the partial count", () => {
  render(StepResult, {
    run: run({
      outcome: "Cancelled",
      new: 0,
      updated: 0,
      partial: 40,
      title: "Annullato: ho fermato la copia.",
      caption: "Annullato.",
      resultNote: "40 file copiati (parziale). I file già copiati restano in B.",
      partialText: "40 file copiati (parziale)",
    }),
    failure: null,
    ...handlers,
  });

  expect(screen.getByRole("heading").textContent).toBe("Annullato: ho fermato la copia.");
  expect(screen.getByText("40 file copiati (parziale). I file già copiati restano in B.")).toBeTruthy();
  expect(screen.getAllByText(/40 file/)).toHaveLength(1);
});

test("cancelled before any file was copied", () => {
  render(StepResult, {
    run: run({
      outcome: "Cancelled",
      partial: 0,
      title: "Annullato: ho fermato la copia.",
      resultNote: NOTHING_COPIED,
    }),
    failure: null,
    ...handlers,
  });

  expect(screen.getByText(NOTHING_COPIED)).toBeTruthy();
  expect(screen.queryByText(/restano in B/)).toBeNull();
});

test("error before any file was copied", () => {
  render(StepResult, { run: failed({ partial: 0, resultNote: NOTHING_COPIED }), failure: null, ...handlers });

  expect(screen.getByText(NOTHING_COPIED)).toBeTruthy();
});

test("error after copying 40 files shows Rust's note next to the message", () => {
  render(StepResult, {
    run: failed({ partial: 40, resultNote: "40 file copiati (parziale)" }),
    failure: null,
    ...handlers,
  });

  expect(screen.getByText(generic.advice)).toBeTruthy();
  expect(screen.getByText("40 file copiati (parziale)")).toBeTruthy();
});

test("a success has no partial line", () => {
  render(StepResult, { run: run(), failure: null, ...handlers });

  expect(screen.queryByText(/parziale/)).toBeNull();
});

const buttons = () => screen.getAllByRole("button").map((b) => b.textContent?.trim());

test("OK: Torna all’elenco is the only button, with the log and history links", () => {
  render(StepResult, { run: run(), failure: null, ...handlers });

  expect(buttons()).toEqual(["Torna all’elenco", "Apri il registro", "Vai allo storico"]);
});

test("Errore and Annullato offer Riprova first, then Torna all’elenco", () => {
  const { unmount } = render(StepResult, { run: run({ outcome: "Error" }), failure: null, ...handlers });
  expect(buttons()).toEqual(["Riprova", "Torna all’elenco", "Apri il registro", "Vai allo storico"]);
  unmount();

  render(StepResult, { run: run({ outcome: "Cancelled", partial: 3 }), failure: null, ...handlers });
  expect(buttons()).toEqual(["Riprova", "Torna all’elenco", "Apri il registro", "Vai allo storico"]);
});

test("an unreachable share: the reason in the title, what to do under it", () => {
  const explanation = {
    title: String.raw`Non completato: la cartella di rete \\NAS non è raggiungibile.`,
    advice: "Controlla la connessione e riprova.",
  };
  render(StepResult, {
    run: run({ outcome: "Error", explanation, title: explanation.title }),
    failure: null,
    ...handlers,
  });

  expect(screen.getByRole("heading").textContent).toBe(explanation.title);
  expect(screen.getByText(explanation.advice)).toBeTruthy();
});

test("Riprova, Torna all’elenco and Vai allo storico call back", async () => {
  const calls: string[] = [];
  render(StepResult, {
    run: run({ outcome: "Error" }),
    failure: null,
    onhome: () => calls.push("home"),
    onretry: () => calls.push("retry"),
    onhistory: () => calls.push("history"),
  });

  await fireEvent.click(screen.getByRole("button", { name: "Riprova" }));
  await fireEvent.click(screen.getByRole("button", { name: "Torna all’elenco" }));
  await fireEvent.click(screen.getByRole("button", { name: "Vai allo storico" }));

  expect(calls).toEqual(["retry", "home", "history"]);
});

test("Apri il registro opens the log viewer of the run, which Chiudi closes", async () => {
  invokeMock.mockResolvedValue({ text: "ERROR 53 …", errorLines: [0] });
  render(StepResult, {
    run: run({ outcome: "Error", logPath: String.raw`C:\L\logs\x.log` }),
    failure: null,
    ...handlers,
  });

  await fireEvent.click(screen.getByRole("button", { name: "Apri il registro" }));

  expect(await screen.findByRole("dialog", { name: "Registro – Copia di Foto (03/11/2026 12:00)" })).toBeTruthy();
  expect(invokeMock).toHaveBeenCalledWith("read_run_log", { path: String.raw`C:\L\logs\x.log` });
  await fireEvent.click(screen.getByRole("button", { name: "Chiudi" }));
  expect(screen.queryByRole("dialog")).toBeNull();
});

test("when no Run exists there is no log to open, but Riprova is still there", () => {
  render(StepResult, { run: null, failure: { title: "spawn failed", advice: "" }, ...handlers });

  expect(buttons()).toEqual(["Riprova", "Torna all’elenco", "Vai allo storico"]);
});
