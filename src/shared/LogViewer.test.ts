import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import { beforeEach, expect, test, vi } from "vitest";
import LogViewer from "./LogViewer.svelte";
import type { RunReport } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);
beforeEach(() => invokeMock.mockReset());

const run = (over: Partial<RunReport> = {}): RunReport => ({
  syncName: "Copia di Foto",
  source: String.raw`D:\Foto`,
  destination: String.raw`E:\Backup`,
  mode: "Copy",
  new: 12,
  updated: 3,
  deleted: 0,
  durationMs: 125_000,
  outcome: "Ok",
  startedAt: "2026-11-03T12:00:00+01:00",
  logPath: String.raw`C:\L\logs\x.log`,
  explanation: null,
  ...over,
});

const show = (r: RunReport, log = { text: "riga", errorLines: [] as number[] }) => {
  invokeMock.mockResolvedValue(log);
  return render(LogViewer, { run: r, onclose: () => {} });
};

const summary = () => within(screen.getByRole("region", { name: "Riepilogo" }));

test("the summary shows the outcome, the mode and the counts of the run above the log", async () => {
  show(run({ mode: "Mirror", deleted: 5 }));

  expect(summary().getByText("OK").classList.contains("ok")).toBe(true);
  expect(summary().getByText("Specchio").classList.contains("mirror")).toBe(true);
  for (const [label, value] of [
    ["Nuovi", "12"],
    ["Aggiornati", "3"],
    ["Cancellati", "5"],
    ["Durata", "2 min"],
  ]) {
    expect(summary().getByText(label).nextElementSibling?.textContent).toBe(value);
  }
  expect(await screen.findByText("riga")).toBeTruthy();
});

test("a stopped or failed run shows the files copied so far instead of the new and updated counts", () => {
  show(run({ outcome: "Cancelled", partial: 40 }));

  expect(summary().getByText("Annullato")).toBeTruthy();
  expect(summary().getByText("40 file copiati (parziale)")).toBeTruthy();
  expect(summary().queryByText("Nuovi")).toBeNull();
});

test("an Errore run shows the explanation Rust gave; an OK run shows none", () => {
  const title = String.raw`Non completato: non trovo la cartella B (E:\Backup).`;
  const { unmount } = show(run({ outcome: "Error", explanation: { title, advice: "Il disco è collegato?" } }));

  expect(summary().getByText("Errore")).toBeTruthy();
  expect(summary().getByText(title)).toBeTruthy();
  expect(summary().getByText("Il disco è collegato?")).toBeTruthy();

  unmount();
  show(run());
  expect(summary().queryByText("Il disco è collegato?")).toBeNull();
});

test("when the log is gone the summary stays and the log says so", async () => {
  invokeMock.mockRejectedValueOnce({ title: "Il registro di questa copia non è più disponibile.", advice: "" });
  render(LogViewer, { run: run(), onclose: () => {} });

  expect(await screen.findByText("Il registro di questa copia non è più disponibile.")).toBeTruthy();
  expect(summary().getByText("Nuovi")).toBeTruthy();
});

test("the error lines of the log get the error style, the others do not", async () => {
  show(run(), { text: "uno\ndue\ntre\nquattro\ncinque", errorLines: [1, 3] });

  const due = await screen.findByText("due");
  expect(due.classList.contains("error-line")).toBe(true);
  expect(screen.getByText("quattro").classList.contains("error-line")).toBe(true);
  expect(document.querySelectorAll(".error-line")).toHaveLength(2);
  // The text itself is untouched, line breaks included.
  expect(screen.getByRole("dialog").querySelector("pre")?.textContent).toBe("uno\ndue\ntre\nquattro\ncinque");
});

test("Vai al primo errore appears only when the log has an error line, and focuses the first one", async () => {
  const { unmount } = show(run(), { text: "uno\ndue\ntre\nquattro", errorLines: [2, 3] });
  const jump = await screen.findByRole("button", { name: "Vai al primo errore" });

  await fireEvent.click(jump);

  expect(document.activeElement).toBe(screen.getByText("tre"));

  unmount();
  show(run(), { text: "uno\ndue", errorLines: [] });
  await screen.findByText("uno", { exact: false });
  expect(screen.queryByRole("button", { name: "Vai al primo errore" })).toBeNull();
});

test("Tab from a focused error line wraps to the first control instead of leaving the dialog", async () => {
  show(run(), { text: "uno\ndue", errorLines: [1] });
  await fireEvent.click(await screen.findByRole("button", { name: "Vai al primo errore" }));

  await fireEvent.keyDown(window, { key: "Tab" });

  expect(document.activeElement).toBe(screen.getByRole("button", { name: "Chiudi" }));
});
