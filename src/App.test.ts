import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, expect, test, vi } from "vitest";
import App from "./App.svelte";
import type { Preview, Run } from "./lib/types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const invokeMock = vi.mocked(invoke);

const preview = (n: number, u: number): Preview => ({
  new: { count: n, files: ["sub\\a.jpg"] },
  updated: { count: u, files: u ? ["b.pdf"] : [] },
  deleted: { count: 0, files: [] },
  bytes: 3_000_000,
});

const doneRun: Run = {
  syncName: "Copia di Foto",
  source: "D:\\Foto",
  destination: "E:\\Backup",
  mode: "Copy",
  new: 2,
  updated: 1,
  deleted: 0,
  durationMs: 2_000,
  outcome: "Ok",
  startedAt: "2026-11-03T12:00:00+01:00",
  logPath: "C:\\L\\logs\\x.log",
};

const click = (target: string | RegExp | HTMLElement) =>
  fireEvent.click(typeof target === "object" && "tagName" in target ? target : screen.getByRole("button", { name: target }));
const typeInto = (label: RegExp, value: string) =>
  fireEvent.input(screen.getByLabelText(label), { target: { value } });

async function goToPreview() {
  await click("+ Nuova sincronizzazione");
  await typeInto(/Cartella da copiare/, "D:\\Foto");
  await click("Avanti");
  await typeInto(/Cartella dove metto la copia/, "E:\\Backup");
  await click("Avanti");
  await click("Avanti"); // Opzioni
}

beforeEach(() => {
  invokeMock.mockReset();
  vi.mocked(open).mockReset();
});

test("shell: sections, a way to start, and the schema docked on the side", () => {
  render(App);

  expect(screen.getByRole("button", { name: "Sincronizzazioni" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Storico" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "+ Nuova sincronizzazione" })).toBeTruthy();
  expect(screen.getByRole("complementary", { name: "Schema di cosa succede" })).toBeTruthy();
});

test("A and B through Opzioni: the preview and the schema show the same counts", async () => {
  invokeMock.mockResolvedValueOnce(preview(148, 3));
  render(App);

  await goToPreview();

  expect(invokeMock).toHaveBeenCalledWith("preview_sync", {
    source: "D:\\Foto",
    destination: "E:\\Backup",
    mode: "Copy",
  });
  expect(await screen.findByText("Da copiare (nuovi)")).toBeTruthy();
  expect(screen.getByText("Da copiare (nuovi)").nextElementSibling?.textContent).toBe("148");
  expect(screen.getByText("Da aggiornare").nextElementSibling?.textContent).toBe("3");

  const schema = screen.getByRole("complementary");
  expect(schema.textContent).toContain("D:\\Foto");
  expect(schema.textContent).toContain("E:\\Backup");
  expect(screen.getByText("148", { selector: "aside b" })).toBeTruthy();
  expect(screen.getByText("3", { selector: "aside b" })).toBeTruthy();
  expect(screen.getByText("Nessun file viene cancellato in B")).toBeTruthy();
});

test("Sì, avvia runs with the previewed counts and says Fatto", async () => {
  invokeMock.mockResolvedValueOnce(preview(2, 1)).mockResolvedValueOnce(doneRun);
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  expect(await screen.findByText("Fatto: 2 file copiati, 1 aggiornato in < 1 min")).toBeTruthy();
  expect(invokeMock).toHaveBeenLastCalledWith("run_sync", {
    source: "D:\\Foto",
    destination: "E:\\Backup",
    mode: "Copy",
    counts: { new: 2, updated: 1, deleted: 0 },
  });
});

test("while the copy runs the UI says Sto copiando…", async () => {
  invokeMock.mockResolvedValueOnce(preview(2, 1)).mockReturnValueOnce(new Promise(() => {}));
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  expect(screen.getByRole("heading", { name: "Sto copiando…" })).toBeTruthy();
});

test("a run that ends in Error says Non completato with the raw message", async () => {
  invokeMock
    .mockResolvedValueOnce(preview(2, 1))
    .mockResolvedValueOnce({ ...doneRun, outcome: "Error", error: "robocopy ha restituito il codice 16." });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  expect(await screen.findByRole("heading", { name: "Non completato" })).toBeTruthy();
  expect(screen.getByText("robocopy ha restituito il codice 16.")).toBeTruthy();
});

test("nothing to do: B è già aggiornato and no Sì, avvia", async () => {
  invokeMock.mockResolvedValueOnce(preview(0, 0));
  render(App);

  await goToPreview();

  expect(await screen.findByText("B è già aggiornato")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
});

test("a failed preview shows the error and no Sì, avvia", async () => {
  invokeMock.mockRejectedValueOnce("robocopy ha restituito il codice 16.");
  render(App);

  await goToPreview();

  expect((await screen.findByRole("alert")).textContent).toContain("robocopy ha restituito il codice 16.");
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
});

test("an empty folder blocks Avanti with a message", async () => {
  render(App);
  await click("+ Nuova sincronizzazione");

  await click("Avanti");

  expect(screen.getByRole("alert").textContent).toContain("Scegli prima la cartella da copiare.");
  expect(screen.getByLabelText(/Cartella da copiare/)).toBeTruthy();
});

test("Indietro goes back keeping what was typed; No, torna all’inizio returns to the start", async () => {
  invokeMock.mockResolvedValueOnce(preview(1, 0));
  render(App);
  await goToPreview();
  await screen.findByText("Da copiare (nuovi)");

  await click("Indietro"); // to Opzioni
  await click("Indietro"); // to Dove
  expect((screen.getByLabelText(/Cartella dove metto la copia/) as HTMLInputElement).value).toBe("E:\\Backup");

  invokeMock.mockResolvedValueOnce(preview(1, 0));
  await click("Avanti");
  await click("Avanti");
  await click(await screen.findByRole("button", { name: "No, torna all’inizio" }));

  expect(screen.getByRole("button", { name: "+ Nuova sincronizzazione" })).toBeTruthy();
});

test("Sfoglia… fills the path with the folder chosen in the dialog", async () => {
  vi.mocked(open).mockResolvedValueOnce("D:\\Musica");
  render(App);
  await click("+ Nuova sincronizzazione");

  await click("Sfoglia…");

  expect(open).toHaveBeenCalledWith({ directory: true });
  await vi.waitFor(() =>
    expect((screen.getByLabelText(/Cartella da copiare/) as HTMLInputElement).value).toBe("D:\\Musica"),
  );
});

test("the Storico tab is there for S4 to fill", async () => {
  render(App);

  await click("Storico");

  expect(screen.getByRole("heading", { name: "Storico" })).toBeTruthy();
});
