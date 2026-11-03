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

const props = { scanning: false, error: "", onstart: () => {}, onstop: () => {} };

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

test("B è già aggiornato: says so and offers no start button", () => {
  render(StepPreview, { ...props, preview: preview(0, 0) });

  expect(screen.getByText("B è già aggiornato")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
});

test("while scanning it reassures that nothing is copied yet and offers no start", () => {
  render(StepPreview, { ...props, scanning: true, preview: null });

  expect(screen.getByText(/Non copio ancora niente/)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
});

test("a failed preview shows the message and offers no start button", () => {
  render(StepPreview, { ...props, preview: null, error: "robocopy ha restituito il codice 16." });

  expect(screen.getByRole("alert").textContent).toContain("robocopy ha restituito il codice 16.");
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
});

test("the buttons call back", async () => {
  const onstart = vi.fn();
  const onstop = vi.fn();
  render(StepPreview, { ...props, preview: preview(1, 0), onstart, onstop });

  await fireEvent.click(screen.getByRole("button", { name: "Sì, avvia" }));
  await fireEvent.click(screen.getByRole("button", { name: "No, torna all’inizio" }));

  expect(onstart).toHaveBeenCalledOnce();
  expect(onstop).toHaveBeenCalledOnce();
});
