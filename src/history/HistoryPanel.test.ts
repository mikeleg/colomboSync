import { render, screen, within } from "@testing-library/svelte";
import { expect, test } from "vitest";
import { readFileSync } from "node:fs";
import HistoryPanel from "./HistoryPanel.svelte";
import type { HistorySummary } from "./types";

const theme = readFileSync("src/theme.css", "utf8");

const summary = (over: Partial<HistorySummary> = {}): HistorySummary => ({
  runs: 9,
  ok: 6,
  failed: 2,
  cancelled: 1,
  filesCopied: 12345,
  filesDeleted: 17,
  syncs: [
    {
      id: 1,
      name: "Documenti di lavoro",
      lastSuccessText: "Ultima copia riuscita: 02/11/2026 21:30",
      warning: "L’ultima copia è finita con un errore il 04/11/2026 10:00.",
    },
    { id: 2, name: "Foto di Anna", lastSuccessText: "Ultima copia riuscita: 03/11/2026 08:05", warning: null },
    { id: 3, name: "Musica", lastSuccessText: "Nessuna copia riuscita finora.", warning: null },
  ],
  ...over,
});

const section = (name: string) => screen.getByRole("heading", { name }).closest("section")!;
const number = (label: string) => screen.getByText(label).previousElementSibling?.textContent;

test("it is no longer called Come leggere lo storico: Riepilogo, Per sincronizzazione and the legend are separate sections", () => {
  render(HistoryPanel, { summary: summary() });

  expect(screen.queryByText("Come leggere lo storico")).toBeNull();
  expect(section("Riepilogo")).toBeTruthy();
  expect(section("Per sincronizzazione")).toBeTruthy();
  expect(section("Cosa vogliono dire le etichette")).toBeTruthy();
});

test("Riepilogo shows runs, ok, with error, cancelled, files copied and files deleted in B", () => {
  render(HistoryPanel, { summary: summary() });

  expect(number("Copie fatte")).toBe("9");
  expect(number("Riuscite")).toBe("6");
  expect(number("Con errore")).toBe("2");
  expect(number("Annullate")).toBe("1");
  expect(number("File copiati")).toBe("12.345");
  expect(number("File cancellati in B")).toBe("17");
});

test("Per sincronizzazione: name, last good copy or none yet, and the warning from Rust", () => {
  render(HistoryPanel, { summary: summary() });
  const list = within(section("Per sincronizzazione"));

  expect(list.getByText("Documenti di lavoro")).toBeTruthy();
  expect(list.getByText("Ultima copia riuscita: 02/11/2026 21:30")).toBeTruthy();
  expect(list.getByText("L’ultima copia è finita con un errore il 04/11/2026 10:00.")).toBeTruthy();
  expect(list.getByText("Ultima copia riuscita: 03/11/2026 08:05")).toBeTruthy();
  expect(list.getByText("Nessuna copia riuscita finora.")).toBeTruthy();
  expect(list.getAllByText(/finita con un errore/)).toHaveLength(1);
});

test("the legend explains OK, Errore, Annullato and Specchio", () => {
  render(HistoryPanel, { summary: summary() });
  const legend = within(section("Cosa vogliono dire le etichette"));

  expect(legend.getByText("La copia è andata a buon fine.")).toBeTruthy();
  expect(
    legend.getByText(/Interrotta per un problema \(di solito un disco o la rete\)\. I file già copiati restano\./),
  ).toBeTruthy();
  expect(legend.getByText("Fermata a mano.")).toBeTruthy();
  expect(legend.getByText("In quella copia erano stati cancellati da B i file in più.")).toBeTruthy();
  for (const chip of ["OK", "Errore", "Annullato", "Specchio"]) expect(legend.getByText(chip)).toBeTruthy();
});

test("the Specchio chip has its own colour, not the one of Errore", () => {
  render(HistoryPanel, { summary: summary() });
  const legend = within(section("Cosa vogliono dire le etichette"));

  expect(legend.getByText("Specchio").classList).toContain("mirror");
  expect(legend.getByText("Specchio").classList).not.toContain("err");
  expect(legend.getByText("Errore").classList).toContain("err");
  // theme.css: .chip.mirror is not painted with the danger tokens, and the theme defines its tokens (light and both dark blocks).
  expect(theme).not.toMatch(/\.chip\.err,\s*\.chip\.mirror/);
  expect(theme.match(/--mirror:/g)).toHaveLength(3);
  expect(theme.match(/--on-mirror:/g)).toHaveLength(3);
});

test("empty history: a friendly message instead of zeros, and the legend stays", () => {
  render(HistoryPanel, {
    summary: summary({ runs: 0, ok: 0, failed: 0, cancelled: 0, filesCopied: 0, filesDeleted: 0, syncs: [] }),
  });

  expect(
    screen.getByText(
      "Non hai ancora fatto nessuna copia. Quando ne farai una, qui vedrai i numeri e lo stato di ogni sincronizzazione.",
    ),
  ).toBeTruthy();
  expect(screen.queryByText("Copie fatte")).toBeNull();
  expect(screen.queryByRole("heading", { name: "Per sincronizzazione" })).toBeNull();
  expect(section("Cosa vogliono dire le etichette")).toBeTruthy();
});

test("while the summary is not there (loading or failed) only the legend shows", () => {
  render(HistoryPanel, { summary: null });

  expect(screen.queryByRole("heading", { name: "Riepilogo" })).toBeNull();
  expect(screen.queryByText(/Non hai ancora fatto nessuna copia/)).toBeNull();
  expect(section("Cosa vogliono dire le etichette")).toBeTruthy();
});

test("runs without a sync of their own: Riepilogo shows, Per sincronizzazione is left out", () => {
  render(HistoryPanel, { summary: summary({ syncs: [] }) });

  expect(section("Riepilogo")).toBeTruthy();
  expect(screen.queryByRole("heading", { name: "Per sincronizzazione" })).toBeNull();
});
