import { render, screen } from "@testing-library/svelte";
import { expect, test } from "vitest";
import type { Run } from "../types";
import StepResult from "./StepResult.svelte";

const run = (over: Partial<Run> = {}): Run => ({
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
  ...over,
});

test("success: Fatto with copied and updated counts and the duration", () => {
  render(StepResult, { run: run(), failure: "", onhome: () => {} });

  expect(screen.getByRole("heading").textContent).toBe("Fatto: 12 file copiati, 3 aggiornati in < 1 min");
  expect(screen.getByRole("button", { name: "Torna all’inizio" })).toBeTruthy();
});

test("success uses the singular", () => {
  render(StepResult, { run: run({ new: 1, updated: 1 }), failure: "", onhome: () => {} });

  expect(screen.getByRole("heading").textContent).toBe("Fatto: 1 file copiato, 1 aggiornato in < 1 min");
});

test("error outcome: Non completato with the raw robocopy message", () => {
  render(StepResult, {
    run: run({ outcome: "Error", error: "robocopy ha restituito il codice 16." }),
    failure: "",
    onhome: () => {},
  });

  expect(screen.getByRole("heading").textContent).toBe("Non completato");
  expect(screen.getByText("robocopy ha restituito il codice 16.")).toBeTruthy();
  expect(screen.queryByText(/^Fatto/)).toBeNull();
});

test("a command failure before any Run exists is shown as not completed", () => {
  render(StepResult, { run: null, failure: "spawn failed", onhome: () => {} });

  expect(screen.getByRole("heading").textContent).toBe("Non completato");
  expect(screen.getByText("spawn failed")).toBeTruthy();
});

test("cancelled: Annullato with what stays in B, and the partial count", () => {
  render(StepResult, {
    run: run({ outcome: "Cancelled", new: 0, updated: 0, partial: 40 }),
    failure: "",
    onhome: () => {},
  });

  expect(screen.getByRole("heading").textContent).toBe("Annullato: ho fermato la copia.");
  expect(screen.getByText("40 file copiati (parziale). I file già copiati restano in B.")).toBeTruthy();
  expect(screen.getAllByText(/40 file/)).toHaveLength(1);
});

test("cancelled before any file was copied", () => {
  render(StepResult, { run: run({ outcome: "Cancelled", partial: 0 }), failure: "", onhome: () => {} });

  expect(screen.getByText("Nessun file è stato copiato.")).toBeTruthy();
  expect(screen.queryByText(/restano in B/)).toBeNull();
});

test("error before any file was copied", () => {
  render(StepResult, { run: run({ outcome: "Error", error: "robocopy ha restituito il codice 16.", partial: 0 }), failure: "", onhome: () => {} });

  expect(screen.getByText("Nessun file è stato copiato.")).toBeTruthy();
});

test("error after copying 40 files says 40 file copiati (parziale) next to the message", () => {
  render(StepResult, {
    run: run({ outcome: "Error", error: "robocopy ha restituito il codice 16.", partial: 40 }),
    failure: "",
    onhome: () => {},
  });

  expect(screen.getByText("robocopy ha restituito il codice 16.")).toBeTruthy();
  expect(screen.getByText("40 file copiati (parziale)")).toBeTruthy();
});

test("a success has no partial line", () => {
  render(StepResult, { run: run(), failure: "", onhome: () => {} });

  expect(screen.queryByText(/parziale/)).toBeNull();
});
