import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, expect, test, vi } from "vitest";
import App from "./App.svelte";
import type { Preview, Run } from "./lib/types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

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

/** Answers the Rust commands the wizard uses; the folder picker always sees an empty computer. */
function backend(handlers: { preview_sync?: () => unknown; run_sync?: () => unknown }) {
  invokeMock.mockImplementation(async (cmd) => {
    if (cmd === "browse_folders") return { parent: "", folders: [], error: null };
    const handler = handlers[cmd as keyof typeof handlers];
    if (!handler) throw new Error(`unexpected command ${cmd}`);
    return handler();
  });
}

const click = (target: string | RegExp | HTMLElement) =>
  fireEvent.click(typeof target === "object" && "tagName" in target ? target : screen.getByRole("button", { name: target }));
const typeInto = (label: RegExp, value: string) =>
  fireEvent.input(screen.getByLabelText(label), { target: { value } });

async function goToPreview() {
  await click("+ Nuova sincronizzazione");
  await typeInto(/Percorso/, "D:\\Foto");
  await click("Avanti");
  await typeInto(/Percorso/, "E:\\Backup");
  await click("Avanti");
  await click("Avanti"); // Opzioni
}

beforeEach(() => {
  invokeMock.mockReset();
});

test("shell: brand, theme switch, sections, a way to start, and the schema docked on the side", () => {
  backend({});
  render(App);

  expect(screen.getByText("ColomboSync")).toBeTruthy();
  expect(screen.getByText("Copie di sicurezza senza sorprese")).toBeTruthy();
  expect(screen.getByRole("group", { name: "Tema" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Sincronizzazioni" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Storico" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "+ Nuova sincronizzazione" })).toBeTruthy();
  expect(screen.getByRole("complementary", { name: "Schema di cosa succede" })).toBeTruthy();
});

test("A and B through Opzioni: the preview and the schema show the same counts", async () => {
  backend({ preview_sync: () => preview(148, 3) });
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
  backend({ preview_sync: () => preview(2, 1), run_sync: () => doneRun });
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
  backend({ preview_sync: () => preview(2, 1), run_sync: () => new Promise(() => {}) });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  expect(screen.getByRole("heading", { name: "Sto copiando…" })).toBeTruthy();
});

test("a run that ends in Error says Non completato with the raw message", async () => {
  backend({
    preview_sync: () => preview(2, 1),
    run_sync: () => ({ ...doneRun, outcome: "Error", error: "robocopy ha restituito il codice 16." }),
  });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  expect(await screen.findByRole("heading", { name: "Non completato" })).toBeTruthy();
  expect(screen.getByText("robocopy ha restituito il codice 16.")).toBeTruthy();
});

test("nothing to do: B è già aggiornato, an Indietro link and no Sì, avvia", async () => {
  backend({ preview_sync: () => preview(0, 0) });
  render(App);

  await goToPreview();

  expect(await screen.findByText("B è già aggiornato")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
  expect(screen.getByRole("button", { name: "Indietro" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Avanti" })).toBeNull();
});

test("a failed preview shows the error, a way back and no Sì, avvia", async () => {
  backend({
    preview_sync: () => {
      throw "robocopy ha restituito il codice 16.";
    },
  });
  render(App);

  await goToPreview();

  expect((await screen.findByRole("alert")).textContent).toContain("robocopy ha restituito il codice 16.");
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
  expect(screen.getByRole("button", { name: "Indietro" })).toBeTruthy();
});

test("an empty folder blocks Avanti with a message and opens the manual input", async () => {
  backend({});
  render(App);
  await click("+ Nuova sincronizzazione");

  await click("Avanti");

  expect(screen.getByRole("alert").textContent).toContain("Scegli prima la cartella da copiare.");
  expect(screen.getByText("Preferisci scrivere il percorso a mano?").closest("details")?.open).toBe(true);
});

test("Enter in the manual input behaves as Avanti", async () => {
  backend({});
  render(App);
  await click("+ Nuova sincronizzazione");
  await typeInto(/Percorso/, "D:\\Foto");

  await fireEvent.keyDown(screen.getByLabelText(/Percorso/), { key: "Enter" });

  expect(screen.getByRole("heading", { name: "Dove copio?" })).toBeTruthy();
});

test("Indietro, cambio qualcosa goes back to Opzioni keeping what was typed; No, salva e basta returns to the start", async () => {
  backend({ preview_sync: () => preview(1, 0) });
  render(App);
  await goToPreview();
  await screen.findByText("Da copiare (nuovi)");
  expect(screen.queryByRole("button", { name: "Avanti" })).toBeNull();

  await click("Indietro, cambio qualcosa"); // to Opzioni
  await click("Indietro"); // to Dove
  expect((screen.getByLabelText(/Percorso/) as HTMLInputElement).value).toBe("E:\\Backup");

  await click("Avanti");
  await click("Avanti");
  await click(await screen.findByRole("button", { name: "No, salva e basta" }));

  expect(screen.getByRole("button", { name: "+ Nuova sincronizzazione" })).toBeTruthy();
});

test("the Storico tab is there for S4 to fill", async () => {
  backend({});
  render(App);

  await click("Storico");

  expect(screen.getByRole("heading", { name: "Storico" })).toBeTruthy();
});
