import { fireEvent, render, screen, within } from "@testing-library/svelte";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, expect, test, vi } from "vitest";
import History from "./History.svelte";
import type { Run } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

const run = (over: Partial<Run> = {}): Run => ({
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
  ...over,
});

beforeEach(() => invokeMock.mockReset());

test("lead and one row per run, in the order given (newest first)", () => {
  render(History, {
    runs: [run({ syncName: "Recente", startedAt: "2026-11-03T08:05:00+01:00" }), run({ syncName: "Vecchia" })],
    status: "ready",
  });

  expect(screen.getByText("Tutte le copie fatte, dalla più recente. Ogni riga ha il suo registro completo.")).toBeTruthy();
  const rows = screen.getAllByRole("listitem");
  expect(rows.map((r) => within(r).getByRole("strong").textContent)).toEqual(["Recente", "Vecchia"]);
  expect(rows[0].textContent).toContain("03/11/2026 08:05");
  expect(rows[1].textContent).toContain("02/11/2026 21:30");
});

test("a row shows mode and outcome chips, counts and duration", () => {
  render(History, { runs: [run({ new: 12000, updated: 3, deleted: 5, mode: "Mirror", outcome: "Error" })], status: "ready" });

  const row = screen.getByRole("listitem");
  expect(within(row).getByText("Specchio")).toBeTruthy();
  expect(within(row).getByText("Errore")).toBeTruthy();
  expect(within(row).getByText("Nuovi").nextElementSibling?.textContent).toBe("12.000");
  expect(within(row).getByText("Aggiornati").nextElementSibling?.textContent).toBe("3");
  expect(within(row).getByText("Cancellati").nextElementSibling?.textContent).toBe("5");
  expect(within(row).getByText("Durata").nextElementSibling?.textContent).toBe("1 min");
});

test("outcome chips: OK, Errore, Annullato; Copia for copy mode", () => {
  render(History, { runs: [run({ outcome: "Ok" }), run({ outcome: "Cancelled" })], status: "ready" });

  const [first, second] = screen.getAllByRole("listitem");
  expect(within(first).getByText("OK")).toBeTruthy();
  expect(within(first).getByText("Copia")).toBeTruthy();
  expect(within(second).getByText("Annullato")).toBeTruthy();
});

test("while the history is being read there are no rows, only a short message", () => {
  render(History, { runs: [], status: "loading" });

  expect(screen.getByText("Sto leggendo lo storico…")).toBeTruthy();
  expect(screen.queryByRole("listitem")).toBeNull();
});

test("when the history cannot be read, a short message says so", () => {
  render(History, { runs: [], status: "error" });

  expect(screen.getByRole("alert").textContent).toContain("Non riesco a leggere lo storico.");
});

test("Registro opens the log modal of that run; Chiudi closes it", async () => {
  invokeMock.mockResolvedValueOnce("riga di log");
  render(History, {
    runs: [run({ syncName: "Altra", logPath: String.raw`C:\L\logs\b.log` })],
    status: "ready",
  });

  await fireEvent.click(screen.getByRole("button", { name: "Registro" }));

  expect(invokeMock).toHaveBeenCalledWith("read_run_log", { path: String.raw`C:\L\logs\b.log` });
  expect(screen.getByRole("dialog", { name: "Registro – Altra (02/11/2026 21:30)" })).toBeTruthy();
  expect(await screen.findByText("riga di log")).toBeTruthy();

  await fireEvent.click(screen.getByRole("button", { name: "Chiudi" }));

  expect(screen.queryByRole("dialog")).toBeNull();
});

test("Esc closes the log modal", async () => {
  invokeMock.mockResolvedValueOnce("x");
  render(History, { runs: [run()], status: "ready" });
  await fireEvent.click(screen.getByRole("button", { name: "Registro" }));

  await fireEvent.keyDown(window, { key: "Escape" });

  expect(screen.queryByRole("dialog")).toBeNull();
});

test("closing the log modal gives the focus back to the Registro button", async () => {
  invokeMock.mockResolvedValueOnce("x");
  render(History, { runs: [run()], status: "ready" });
  const opener = screen.getByRole("button", { name: "Registro" });
  opener.focus();
  await fireEvent.click(opener);
  expect(document.activeElement).toBe(screen.getByRole("button", { name: "Chiudi" }));

  await fireEvent.keyDown(window, { key: "Escape" });

  expect(document.activeElement).toBe(opener);
});

test("Tab stays inside the log modal", async () => {
  invokeMock.mockResolvedValueOnce("x");
  render(History, { runs: [run()], status: "ready" });
  await fireEvent.click(screen.getByRole("button", { name: "Registro" }));
  const close = screen.getByRole("button", { name: "Chiudi" });
  const logText = await screen.findByText("x");

  logText.focus();
  await fireEvent.keyDown(logText, { key: "Tab" });
  expect(document.activeElement).toBe(close);

  await fireEvent.keyDown(close, { key: "Tab", shiftKey: true });
  expect(document.activeElement).toBe(logText);
});

test("a deleted log file says it is no longer available", async () => {
  invokeMock.mockRejectedValueOnce("os error 2");
  render(History, { runs: [run()], status: "ready" });

  await fireEvent.click(screen.getByRole("button", { name: "Registro" }));

  expect(await screen.findByText("Il registro di questa copia non è più disponibile.")).toBeTruthy();
});
