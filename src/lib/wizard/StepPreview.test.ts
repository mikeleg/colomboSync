import { fireEvent, render, screen } from "@testing-library/svelte";
import { expect, test, vi } from "vitest";
import type { Preview } from "../types";
import StepPreview from "./StepPreview.svelte";

const group = (count: number, files: string[] = []) => ({ count, files });
const preview = (n: number, u: number, files = { n: ["a.jpg", "b.jpg"], u: ["c.pdf"] }): Preview => ({
  new: group(n, files.n.slice(0, n)),
  updated: group(u, files.u.slice(0, u)),
  deleted: group(0),
  bytes: 5_000_000,
});

const props = { scanning: false, error: "", onstart: () => {}, onstop: () => {}, onback: () => {} };

test("shows new and updated groups with counts, the files, and the start button", () => {
  render(StepPreview, { ...props, preview: preview(148, 1) });

  expect(screen.getByText("Da copiare (nuovi)").nextElementSibling?.textContent).toBe("148");
  expect(screen.getByText("Da aggiornare").nextElementSibling?.textContent).toBe("1");
  expect(screen.getByText("a.jpg")).toBeTruthy();
  expect(screen.getByText("… e altri 146 file")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Sì, avvia" })).toBeTruthy();
  expect(screen.queryByText("Da cancellare in B")).toBeNull();
});

test("hides a group that has nothing in it", () => {
  render(StepPreview, { ...props, preview: preview(2, 0) });

  expect(screen.getByText("Da copiare (nuovi)")).toBeTruthy();
  expect(screen.queryByText("Da aggiornare")).toBeNull();
});

test("B è già aggiornato: says so, offers no start button and a way back", async () => {
  const onback = vi.fn();
  render(StepPreview, { ...props, preview: preview(0, 0), onback });

  expect(screen.getByText("B è già aggiornato")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();

  await fireEvent.click(screen.getByRole("button", { name: "Indietro" }));
  expect(onback).toHaveBeenCalledOnce();
});

test("while scanning it reassures that nothing is copied yet and offers no start", () => {
  render(StepPreview, { ...props, scanning: true, preview: null });

  expect(screen.getByText(/Non copio ancora niente/)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
});

test("a failed preview shows the message, no start button and a way back", async () => {
  const onback = vi.fn();
  render(StepPreview, { ...props, preview: null, error: "robocopy ha restituito il codice 16.", onback });

  expect(screen.getByRole("alert").textContent).toContain("robocopy ha restituito il codice 16.");
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();

  await fireEvent.click(screen.getByRole("button", { name: "Indietro" }));
  expect(onback).toHaveBeenCalledOnce();
});

test("the three choices after the preview call back: Sì, avvia / No, salva e basta / Indietro, cambio qualcosa", async () => {
  const onstart = vi.fn();
  const onstop = vi.fn();
  const onback = vi.fn();
  render(StepPreview, { ...props, preview: preview(1, 0), onstart, onstop, onback });

  await fireEvent.click(screen.getByRole("button", { name: "Sì, avvia" }));
  await fireEvent.click(screen.getByRole("button", { name: "No, salva e basta" }));
  await fireEvent.click(screen.getByRole("button", { name: "Indietro, cambio qualcosa" }));

  expect(onstart).toHaveBeenCalledOnce();
  expect(onstop).toHaveBeenCalledOnce();
  expect(onback).toHaveBeenCalledOnce();
});

test("a new sync offers to save it: No, salva e basta, and Salva e chiudi when B is already up to date", async () => {
  const onstop = vi.fn();
  const { unmount } = render(StepPreview, { ...props, preview: preview(0, 0), onstop });

  await fireEvent.click(screen.getByRole("button", { name: "Salva e chiudi" }));
  expect(onstop).toHaveBeenCalledOnce();
  expect(screen.queryByRole("button", { name: "Torna all’elenco" })).toBeNull();
  unmount();

  render(StepPreview, { ...props, preview: preview(1, 0) });
  expect(screen.getByRole("button", { name: "No, salva e basta" })).toBeTruthy();
});

test("re-running a saved sync: No, torna all’elenco, no way to change things, Torna all’elenco when up to date", async () => {
  const onstop = vi.fn();
  const { unmount } = render(StepPreview, { ...props, saved: true, preview: preview(1, 0), onstop });

  await fireEvent.click(screen.getByRole("button", { name: "No, torna all’elenco" }));
  expect(onstop).toHaveBeenCalledOnce();
  expect(screen.queryByRole("button", { name: "No, salva e basta" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Indietro, cambio qualcosa" })).toBeNull();
  unmount();

  render(StepPreview, { ...props, saved: true, preview: preview(0, 0) });
  expect(screen.getByRole("button", { name: "Torna all’elenco" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Salva e chiudi" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Indietro" })).toBeNull();
});
