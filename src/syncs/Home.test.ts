import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import { beforeEach, expect, test, vi } from "vitest";
import Home from "./Home.svelte";
import type { SyncSummary } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

const foto: SyncSummary = {
  id: 2,
  name: "Copia di Foto",
  source: String.raw`D:\Foto`,
  destination: String.raw`\\NAS\Backup`,
  mode: "Copy",
  lastRun: { startedAt: "2026-11-02T21:30:00+01:00", outcome: "Ok", note: null },
};
const documenti: SyncSummary = {
  id: 1,
  name: "Documenti ufficio",
  source: String.raw`C:\Users\Anna\Documenti`,
  destination: String.raw`E:\Backup`,
  mode: "Mirror",
  lastRun: null,
};

// The notes come from Rust (error_messages), already in Italian.
const unreachable = String.raw`La cartella di rete \\NAS non era raggiungibile. Controlla che sia collegato e riprova.`;
const stopped = "La copia era stata fermata a mano: puoi riprenderla quando vuoi.";

const props = { onnew: () => {}, onrun: () => {}, onschema: () => {} };

beforeEach(() => {
  invokeMock.mockReset();
});

const card = (name: string) => screen.getByRole("heading", { name }).closest("li") as HTMLElement;

test("lists one card per sync: A → B, mode chip, last time with outcome, or Mai avviata", async () => {
  invokeMock.mockResolvedValue({ syncs: [foto, documenti], setAside: false });
  render(Home, props);

  expect(await screen.findByRole("heading", { name: "Copia di Foto" })).toBeTruthy();
  expect(screen.getByRole("heading", { name: "Le tue sincronizzazioni" })).toBeTruthy();
  expect(
    screen.getByText(
      "Ogni sincronizzazione copia i file da una cartella (A) in un’altra (B). Prima di copiare ti mostro sempre cosa cambierà.",
    ),
  ).toBeTruthy();
  const first = within(card("Copia di Foto"));
  expect(first.getByText(String.raw`D:\Foto`)).toBeTruthy();
  expect(first.getByText(String.raw`\\NAS\Backup`)).toBeTruthy();
  expect(first.getByText("Copia")).toBeTruthy();
  expect(first.getByText("02/11/2026 21:30")).toBeTruthy();
  expect(first.getByText("OK")).toBeTruthy();
  expect(first.getByRole("button", { name: /^Controlla e avvia/ })).toBeTruthy();
  expect(first.queryByText(/raggiungibile|fermata/)).toBeNull(); // an OK run has no note

  const second = within(card("Documenti ufficio"));
  expect(second.getByText("Specchio")).toBeTruthy();
  expect(second.getByText("Mai avviata")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Nuova sincronizzazione" })).toBeTruthy();
});

test("when the last outcome is Errore the button reads Riprova and the chip says Errore", async () => {
  invokeMock.mockResolvedValue({ syncs: [{ ...foto, lastRun: { startedAt: "2026-11-01T18:05:00+01:00", outcome: "Error", note: unreachable } }], setAside: false });
  render(Home, props);

  await screen.findByRole("heading", { name: "Copia di Foto" });
  const c = within(card("Copia di Foto"));
  expect(c.getByText("Errore")).toBeTruthy();
  expect(c.getByText(unreachable)).toBeTruthy();
  expect(c.getByText("01/11/2026 18:05")).toBeTruthy();
  expect(c.getByRole("button", { name: "Riprova Copia di Foto" })).toBeTruthy();
  expect(c.queryByRole("button", { name: /Controlla e avvia/ })).toBeNull();
});

test("a stopped run shows the Annullato chip and keeps Controlla e avvia", async () => {
  invokeMock.mockResolvedValue({ syncs: [{ ...foto, lastRun: { startedAt: "2026-11-01T18:05:00+01:00", outcome: "Cancelled", note: stopped } }], setAside: false });
  render(Home, props);

  await screen.findByRole("heading", { name: "Copia di Foto" });
  expect(within(card("Copia di Foto")).getByText("Annullato")).toBeTruthy();
  expect(within(card("Copia di Foto")).getByText(stopped)).toBeTruthy();
  expect(screen.getByRole("button", { name: /^Controlla e avvia/ })).toBeTruthy();
});

test("no syncs yet: empty state with the dashed Nuova sincronizzazione card", async () => {
  invokeMock.mockResolvedValue({ syncs: [], setAside: false });
  const onnew = vi.fn();
  render(Home, { ...props, onnew });

  await fireEvent.click(await screen.findByRole("button", { name: "Nuova sincronizzazione" }));

  expect(onnew).toHaveBeenCalledOnce();
  expect(screen.queryByRole("listitem")).toBeNull();
  expect(screen.queryByRole("alert")).toBeNull();
});

test("a damaged store set aside by Rust: empty state with a note and the new sync card still enabled", async () => {
  invokeMock.mockResolvedValue({ syncs: [], setAside: true });
  const onnew = vi.fn();
  render(Home, { ...props, onnew });

  expect(await screen.findByText("Le sincronizzazioni salvate non erano leggibili: ne ho messo da parte una copia.")).toBeTruthy();
  const newCard = screen.getByRole("button", { name: "Nuova sincronizzazione" }) as HTMLButtonElement;
  expect(newCard.disabled).toBe(false);
  await fireEvent.click(newCard);
  expect(onnew).toHaveBeenCalledOnce();
  expect(screen.queryByRole("listitem")).toBeNull();
  expect(screen.queryByRole("alert")).toBeNull();
});

test("a store that could not even be set aside: a message, no list, and a new sync cannot be started", async () => {
  invokeMock.mockImplementation(async () => {
    throw "Non riesco a leggere le sincronizzazioni salvate.";
  });
  render(Home, props);

  expect((await screen.findByRole("alert")).textContent).toContain("Non riesco a leggere le sincronizzazioni salvate.");
  expect((screen.getByRole("button", { name: "Nuova sincronizzazione" }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.queryByRole("listitem")).toBeNull();
});

test("the first card is shown in the schema; selecting another shows its A and B with the caption", async () => {
  invokeMock.mockResolvedValue({ syncs: [foto, documenti], setAside: false });
  const onschema = vi.fn();
  render(Home, { ...props, onschema });
  await screen.findByRole("heading", { name: "Copia di Foto" });

  const caption = 'Premi "Controlla e avvia": prima ti mostro cosa cambierà, poi decidi tu.';
  expect(onschema).toHaveBeenLastCalledWith({
    source: foto.source,
    destination: foto.destination,
    counts: null,
    caption,
    mirror: false,
    warnCaption: false,
  });

  await fireEvent.click(card("Documenti ufficio")); // a mirror sync: the schema is in mirror mode

  expect(onschema).toHaveBeenLastCalledWith({
    source: documenti.source,
    destination: documenti.destination,
    counts: null,
    caption,
    mirror: true,
    warnCaption: false,
  });
});

test("Controlla e avvia hands the sync to the app, which opens the preview", async () => {
  invokeMock.mockResolvedValue({ syncs: [foto, documenti], setAside: false });
  const onrun = vi.fn();
  render(Home, { ...props, onrun });
  await screen.findByRole("heading", { name: "Copia di Foto" });

  await fireEvent.click(screen.getByRole("button", { name: "Controlla e avvia Documenti ufficio" }));

  expect(onrun).toHaveBeenCalledWith(documenti);
});
