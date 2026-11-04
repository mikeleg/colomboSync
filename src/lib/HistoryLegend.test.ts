import { render, screen } from "@testing-library/svelte";
import { expect, test } from "vitest";
import HistoryLegend from "./HistoryLegend.svelte";
import type { Run } from "./types";

const run = (over: Partial<Run> = {}): Run => ({
  syncName: "Copia di Foto",
  source: "D:\Foto",
  destination: "E:\Backup",
  mode: "Copy",
  new: 1,
  updated: 0,
  deleted: 0,
  durationMs: 1_000,
  outcome: "Ok",
  startedAt: "2026-11-02T21:30:00+01:00",
  logPath: "C:\L\a.log",
  ...over,
});

test("counters: all runs and the successful ones", () => {
  render(HistoryLegend, {
    runs: [run({ outcome: "Error" }), run(), run({ outcome: "Cancelled" }), run()],
  });

  expect(screen.getByRole("heading", { name: "Come leggere lo storico" })).toBeTruthy();
  expect(screen.getByText("Copie fatte").previousElementSibling?.textContent).toBe("4");
  expect(screen.getByText("Riuscite").previousElementSibling?.textContent).toBe("2");
});

test("legend explains OK, Errore, Annullato and Specchio", () => {
  render(HistoryLegend, { runs: [] });

  expect(screen.getByText("La copia è andata a buon fine.")).toBeTruthy();
  expect(screen.getByText(/Interrotta per un problema \(di solito un disco o la rete\)\. I file già copiati restano\./)).toBeTruthy();
  expect(screen.getByText("Fermata a mano.")).toBeTruthy();
  expect(screen.getByText("In quella copia erano stati cancellati da B i file in più.")).toBeTruthy();
  for (const chip of ["OK", "Errore", "Annullato", "Specchio"]) expect(screen.getByText(chip)).toBeTruthy();
});

test("Ultima copia riuscita is the most recent OK run (list is newest first)", () => {
  render(HistoryLegend, {
    runs: [
      run({ outcome: "Error", startedAt: "2026-11-04T10:00:00+01:00" }),
      run({ startedAt: "2026-11-03T08:05:00+01:00" }),
      run({ startedAt: "2026-11-01T09:00:00+01:00" }),
    ],
  });

  expect(screen.getByText("Ultima copia riuscita: 03/11/2026 08:05.")).toBeTruthy();
});

test("no successful run: no Ultima copia riuscita line", () => {
  render(HistoryLegend, { runs: [run({ outcome: "Error" })] });

  expect(screen.queryByText(/Ultima copia riuscita/)).toBeNull();
});
