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
  blocked: null,
});

const props = {
  scanning: false,
  mirror: false,
  error: null,
  onstart: () => {},
  onstop: () => {},
  onback: () => {},
  onretry: () => {},
};

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

test("a failed preview shows what happened and what to do, no start button, Riprova and a way back", async () => {
  const onback = vi.fn();
  const onretry = vi.fn();
  const error = {
    title: String.raw`Non riesco a controllare le cartelle: la cartella di rete \NAS non è raggiungibile.`,
    advice: "Controlla la connessione e riprova.",
  };
  render(StepPreview, { ...props, preview: null, error, onback, onretry });

  expect(screen.getByRole("alert").textContent).toContain(error.title);
  expect(screen.getByText(error.advice)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();

  await fireEvent.click(screen.getByRole("button", { name: "Riprova" }));
  expect(onretry).toHaveBeenCalledOnce();
  await fireEvent.click(screen.getByRole("button", { name: "Indietro" }));
  expect(onback).toHaveBeenCalledOnce();
});

test("a failure with no advice shows just the title", () => {
  render(StepPreview, { ...props, preview: null, error: { title: "La destinazione non può stare qui.", advice: "" } });

  expect(screen.getByRole("alert").textContent?.trim()).toBe("La destinazione non può stare qui.");
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

const mirrorPreview = (deleted: number, over: Partial<Preview> = {}): Preview => ({
  ...preview(3, 1),
  deleted: group(deleted, ["vecchio.txt", String.raw`sub\altro.txt`].slice(0, deleted)),
  ...over,
});
const REASON = "Motivo deciso da Rust: non si parte.";
const mirrorProps = { ...props, mirror: true };

test("mirror: extras in B are a red group 'Da cancellare in B' and Sì, avvia waits for the consent", async () => {
  const onstart = vi.fn();
  render(StepPreview, { ...mirrorProps, preview: mirrorPreview(12, { new: group(20, ["a"]) }), onstart });

  const grp = screen.getByText("Da cancellare in B");
  expect(grp.nextElementSibling?.textContent).toBe("12");
  expect(grp.closest("details")?.className).toContain("del");
  expect(screen.getByText("vecchio.txt")).toBeTruthy();
  const start = screen.getByRole("button", { name: "Sì, avvia" }) as HTMLButtonElement;
  expect(start.disabled).toBe(true);

  await fireEvent.click(screen.getByLabelText("Ho capito: 12 file verranno cancellati da B"));
  expect(start.disabled).toBe(false);
  await fireEvent.click(start);
  expect(onstart).toHaveBeenCalledOnce();

  await fireEvent.click(screen.getByLabelText("Ho capito: 12 file verranno cancellati da B"));
  expect(start.disabled).toBe(true);
});

test("mirror: the consent is asked again every time the preview is shown", async () => {
  const first = render(StepPreview, { ...mirrorProps, preview: mirrorPreview(2, { new: group(5, ["a"]) }) });
  await fireEvent.click(screen.getByLabelText(/Ho capito/));
  first.unmount();

  render(StepPreview, { ...mirrorProps, preview: mirrorPreview(2, { new: group(5, ["a"]) }) });

  expect((screen.getByLabelText(/Ho capito/) as HTMLInputElement).checked).toBe(false);
  expect((screen.getByRole("button", { name: "Sì, avvia" }) as HTMLButtonElement).disabled).toBe(true);
});

test("mirror with no extra files: says so, no consent checkbox, Sì, avvia is enabled", () => {
  render(StepPreview, { ...mirrorProps, preview: mirrorPreview(0) });

  expect(screen.getByText("In B non ci sono file in più: non verrà cancellato niente.")).toBeTruthy();
  expect(screen.queryByText("Da cancellare in B")).toBeNull();
  expect(screen.queryByRole("checkbox")).toBeNull();
  expect((screen.getByRole("button", { name: "Sì, avvia" }) as HTMLButtonElement).disabled).toBe(false);
});

test("mirror: the summary also counts what will be deleted", () => {
  render(StepPreview, { ...mirrorProps, preview: mirrorPreview(2) });

  expect(screen.getByText(/Ecco cosa succederà in B/).textContent).toContain("2 da cancellare");
});

test("MirrorGuard: when blocked it shows the warning and offers no Sì, avvia and no consent", async () => {
  const onback = vi.fn();
  render(StepPreview, { ...mirrorProps, preview: mirrorPreview(9, { blocked: REASON }), onback });

  expect(screen.getByRole("alert").textContent).toContain(REASON);
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
  expect(screen.queryByRole("checkbox")).toBeNull();

  await fireEvent.click(screen.getByRole("button", { name: "Indietro, cambio qualcosa" }));
  expect(onback).toHaveBeenCalledOnce();
});

test("MirrorGuard: an A that is missing (nothing to list) still shows the warning, not 'B è già aggiornato'", () => {
  render(StepPreview, { ...mirrorProps, preview: { ...preview(0, 0), blocked: REASON } });

  expect(screen.getByRole("alert").textContent).toContain(REASON);
  expect(screen.queryByText("B è già aggiornato")).toBeNull();
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
  expect(screen.getByRole("button", { name: "Indietro, cambio qualcosa" })).toBeTruthy();
});
