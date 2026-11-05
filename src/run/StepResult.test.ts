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
  ...over,
});

// What Rust sends for a failure it has no cause for.
const generic = {
  title: "Non completato: la copia si è fermata per un problema.",
  advice: "Apri il registro per vedere i dettagli.",
};

test("success: Fatto with copied and updated counts and the duration", () => {
  render(StepResult, { run: run(), failure: "", ...handlers });

  expect(screen.getByRole("heading").textContent).toBe("Fatto: 12 file copiati, 3 aggiornati in < 1 min");
  expect(screen.getByRole("button", { name: "Torna all’elenco" })).toBeTruthy();
});

test("success uses the singular", () => {
  render(StepResult, { run: run({ new: 1, updated: 1 }), failure: "", ...handlers });

  expect(screen.getByRole("heading").textContent).toBe("Fatto: 1 file copiato, 1 aggiornato in < 1 min");
});

test("error outcome without a known cause: the generic title and where to look", () => {
  render(StepResult, {
    run: run({ outcome: "Error", explanation: generic }),
    failure: "",
    ...handlers,
  });

  expect(screen.getByRole("heading").textContent).toBe(generic.title);
  expect(screen.getByText(generic.advice)).toBeTruthy();
  expect(screen.queryByText(/^Fatto/)).toBeNull();
});

test("a command failure before any Run exists is shown as not completed", () => {
  render(StepResult, { run: null, failure: "spawn failed", ...handlers });

  expect(screen.getByRole("heading").textContent).toBe("Non completato");
  expect(screen.getByText("spawn failed")).toBeTruthy();
});

test("a mirror run also says how many files were deleted from B", () => {
  render(StepResult, { run: run({ mode: "Mirror", deleted: 12 }), failure: "", ...handlers });

  expect(screen.getByRole("heading").textContent).toBe(
    "Fatto: 12 file copiati, 3 aggiornati, 12 cancellati da B in < 1 min",
  );
});

test("a mirror run that deleted nothing, or one file, reads naturally", () => {
  const { unmount } = render(StepResult, { run: run({ mode: "Mirror", deleted: 0 }), failure: "", ...handlers });
  expect(screen.getByRole("heading").textContent).toBe("Fatto: 12 file copiati, 3 aggiornati in < 1 min");
  unmount();

  render(StepResult, { run: run({ mode: "Mirror", deleted: 1 }), failure: "", ...handlers });
  expect(screen.getByRole("heading").textContent).toContain(", 1 cancellato da B in ");
});

test("cancelled: Annullato with what stays in B, and the partial count", () => {
  render(StepResult, {
    run: run({ outcome: "Cancelled", new: 0, updated: 0, partial: 40 }),
    failure: "",
    ...handlers,
  });

  expect(screen.getByRole("heading").textContent).toBe("Annullato: ho fermato la copia.");
  expect(screen.getByText("40 file copiati (parziale). I file già copiati restano in B.")).toBeTruthy();
  expect(screen.getAllByText(/40 file/)).toHaveLength(1);
});

test("cancelled before any file was copied", () => {
  render(StepResult, { run: run({ outcome: "Cancelled", partial: 0 }), failure: "", ...handlers });

  expect(screen.getByText("Nessun file è stato copiato.")).toBeTruthy();
  expect(screen.queryByText(/restano in B/)).toBeNull();
});

test("error before any file was copied", () => {
  render(StepResult, { run: run({ outcome: "Error", explanation: generic, partial: 0 }), failure: "", ...handlers });

  expect(screen.getByText("Nessun file è stato copiato.")).toBeTruthy();
});

test("error after copying 40 files says 40 file copiati (parziale) next to the message", () => {
  render(StepResult, {
    run: run({ outcome: "Error", explanation: generic, partial: 40 }),
    failure: "",
    ...handlers,
  });

  expect(screen.getByText(generic.advice)).toBeTruthy();
  expect(screen.getByText("40 file copiati (parziale)")).toBeTruthy();
});

test("a success has no partial line", () => {
  render(StepResult, { run: run(), failure: "", ...handlers });

  expect(screen.queryByText(/parziale/)).toBeNull();
});

const buttons = () => screen.getAllByRole("button").map((b) => b.textContent?.trim());

test("OK: Torna all’elenco is the only button, with the log and history links", () => {
  render(StepResult, { run: run(), failure: "", ...handlers });

  expect(buttons()).toEqual(["Torna all’elenco", "Apri il registro", "Vai allo storico"]);
});

test("Errore and Annullato offer Riprova first, then Torna all’elenco", () => {
  const { unmount } = render(StepResult, { run: run({ outcome: "Error" }), failure: "", ...handlers });
  expect(buttons()).toEqual(["Riprova", "Torna all’elenco", "Apri il registro", "Vai allo storico"]);
  unmount();

  render(StepResult, { run: run({ outcome: "Cancelled", partial: 3 }), failure: "", ...handlers });
  expect(buttons()).toEqual(["Riprova", "Torna all’elenco", "Apri il registro", "Vai allo storico"]);
});

test("an unreachable share: the reason in the title, what to do under it", () => {
  const explanation = {
    title: String.raw`Non completato: la cartella di rete \\NAS non è raggiungibile.`,
    advice: "Controlla la connessione e riprova.",
  };
  render(StepResult, { run: run({ outcome: "Error", explanation }), failure: "", ...handlers });

  expect(screen.getByRole("heading").textContent).toBe(explanation.title);
  expect(screen.getByText(explanation.advice)).toBeTruthy();
});

test("Riprova, Torna all’elenco and Vai allo storico call back", async () => {
  const calls: string[] = [];
  render(StepResult, {
    run: run({ outcome: "Error" }),
    failure: "",
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
  invokeMock.mockResolvedValue("ERROR 53 …");
  render(StepResult, { run: run({ outcome: "Error", logPath: String.raw`C:\L\logs\x.log` }), failure: "", ...handlers });

  await fireEvent.click(screen.getByRole("button", { name: "Apri il registro" }));

  expect(await screen.findByRole("dialog", { name: "Registro – Copia di Foto (03/11/2026 12:00)" })).toBeTruthy();
  expect(invokeMock).toHaveBeenCalledWith("read_run_log", { path: String.raw`C:\L\logs\x.log` });
  await fireEvent.click(screen.getByRole("button", { name: "Chiudi" }));
  expect(screen.queryByRole("dialog")).toBeNull();
});

test("when no Run exists there is no log to open, but Riprova is still there", () => {
  render(StepResult, { run: null, failure: "spawn failed", ...handlers });

  expect(buttons()).toEqual(["Riprova", "Torna all’elenco", "Vai allo storico"]);
});

test("a failed run without an explanation (an older one) still says Non completato", () => {
  render(StepResult, { run: run({ outcome: "Error" }), failure: "", ...handlers });

  expect(screen.getByRole("heading").textContent).toBe("Non completato");
});
