import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import { beforeEach, expect, test, vi } from "vitest";
import App from "./App.svelte";
import type { Preview, Run, SyncSummary } from "./lib/types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));

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

const savedSync: SyncSummary = {
  id: 7,
  name: "Foto di Anna",
  source: "D:\\Foto",
  destination: "E:\\Backup",
  mode: "Copy",
  lastRun: { startedAt: "2026-11-02T21:30:00+01:00", outcome: "Ok" },
};

type Handlers = Partial<
  Record<"preview_sync" | "run_sync" | "list_syncs" | "save_sync" | "list_runs" | "read_run_log", () => unknown>
>;

/** Answers the Rust commands the app uses; by default the computer has no folders and no saved syncs. */
function backend(handlers: Handlers = {}) {
  const defaults = {
    browse_folders: () => ({ parent: "", folders: [], error: null }),
    list_syncs: () => [],
    default_sync_name: () => "Copia di Foto",
    save_sync: () => ({ ...savedSync, lastRun: undefined }),
  };
  invokeMock.mockImplementation(async (cmd) => {
    const handler = (handlers as Record<string, () => unknown>)[cmd] ?? (defaults as Record<string, () => unknown>)[cmd];
    if (!handler) throw new Error(`unexpected command ${cmd}`);
    return handler();
  });
}

const calls = (cmd: string) => invokeMock.mock.calls.filter(([c]) => c === cmd);

const click = (target: string | RegExp | HTMLElement) =>
  fireEvent.click(typeof target === "object" && "tagName" in target ? target : screen.getByRole("button", { name: target }));
const typeInto = (label: RegExp, value: string) =>
  fireEvent.input(screen.getByLabelText(label), { target: { value } });

async function goToPreview() {
  await click("Nuova sincronizzazione");
  await typeInto(/Percorso/, "D:\\Foto");
  await click("Avanti");
  await typeInto(/Percorso/, "E:\\Backup");
  await click("Avanti");
  await screen.findByDisplayValue("Copia di Foto");
  await click("Avanti"); // Come la chiamo?
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
  expect(screen.getByRole("button", { name: "Nuova sincronizzazione" })).toBeTruthy();
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
    syncName: "Copia di Foto",
  });
});

test("while the copy runs the UI says Sto copiando…", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => new Promise(() => {}) });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  expect(screen.getByRole("heading", { name: /^Sto copiando/ })).toBeTruthy();
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
  await click("Nuova sincronizzazione");

  await click("Avanti");

  expect(screen.getByRole("alert").textContent).toContain("Scegli prima la cartella da copiare.");
  expect(screen.getByText("Preferisci scrivere il percorso a mano?").closest("details")?.open).toBe(true);
});

test("Enter in the manual input behaves as Avanti", async () => {
  backend({});
  render(App);
  await click("Nuova sincronizzazione");
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
  await click("Indietro"); // to Come la chiamo?
  await click("Indietro"); // to Dove
  expect((screen.getByLabelText(/Percorso/) as HTMLInputElement).value).toBe("E:\\Backup");

  await click("Avanti");
  await click("Avanti");
  await click("Avanti");
  await click(await screen.findByRole("button", { name: "No, salva e basta" }));

  expect(await screen.findByRole("heading", { name: "Le tue sincronizzazioni" })).toBeTruthy();
});

test("Come la chiamo? sits before Opzioni, is prefilled from A and an empty name blocks Avanti", async () => {
  backend();
  render(App);
  await click("Nuova sincronizzazione");
  await typeInto(/Percorso/, "D:\\Foto");
  await click("Avanti");
  await typeInto(/Percorso/, "E:\\Backup");
  await click("Avanti");

  expect(screen.getByRole("heading", { name: "Come la chiamo?" })).toBeTruthy();
  expect(invokeMock).toHaveBeenCalledWith("default_sync_name", { source: "D:\\Foto" });
  const name = (await screen.findByDisplayValue("Copia di Foto")) as HTMLInputElement;

  await fireEvent.input(name, { target: { value: "  " } });
  await click("Avanti");

  expect(screen.getByRole("alert").textContent).toContain("Scrivi un nome per riconoscere questa sincronizzazione.");
  expect(screen.getByRole("heading", { name: "Come la chiamo?" })).toBeTruthy();

  await fireEvent.input(name, { target: { value: "Foto di Anna" } });
  await click("Avanti");
  expect(screen.getByRole("heading", { name: "Opzioni" })).toBeTruthy();
});

test("a name the user typed is not replaced when going back and forth", async () => {
  backend();
  render(App);
  await click("Nuova sincronizzazione");
  await typeInto(/Percorso/, "D:\\Foto");
  await click("Avanti");
  await typeInto(/Percorso/, "E:\\Backup");
  await click("Avanti");
  await fireEvent.input(await screen.findByDisplayValue("Copia di Foto"), { target: { value: "Foto di Anna" } });

  await click("Indietro");
  await click("Avanti");

  expect((screen.getByLabelText("Nome della sincronizzazione") as HTMLInputElement).value).toBe("Foto di Anna");
});

test("the new sync is saved when the run starts, under the name typed, and the run carries that name", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => doneRun });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  await screen.findByText(/Fatto:/);
  expect(calls("save_sync")).toEqual([
    ["save_sync", { name: "Copia di Foto", source: "D:\\Foto", destination: "E:\\Backup", mode: "Copy" }],
  ]);
  expect(invokeMock).toHaveBeenLastCalledWith("run_sync", {
    source: "D:\\Foto",
    destination: "E:\\Backup",
    mode: "Copy",
    counts: { new: 2, updated: 1, deleted: 0 },
    syncName: "Copia di Foto",
  });
});

test("No, salva e basta saves the sync without running it and returns to the list", async () => {
  backend({ preview_sync: () => preview(2, 1) });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "No, salva e basta" }));

  expect(await screen.findByRole("heading", { name: "Le tue sincronizzazioni" })).toBeTruthy();
  expect(calls("save_sync")).toHaveLength(1);
  expect(calls("run_sync")).toHaveLength(0);
});

test("B è già aggiornato: Salva e chiudi saves the new sync and returns to the list", async () => {
  backend({ preview_sync: () => preview(0, 0) });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Salva e chiudi" }));

  expect(await screen.findByRole("heading", { name: "Le tue sincronizzazioni" })).toBeTruthy();
  expect(calls("save_sync")).toHaveLength(1);
});

test("a sync that cannot be saved says so and does not leave the preview", async () => {
  backend({
    preview_sync: () => preview(2, 1),
    save_sync: () => {
      throw "Non riesco a salvare la sincronizzazione: disco pieno.";
    },
  });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "No, salva e basta" }));

  expect((await screen.findByRole("alert")).textContent).toContain("Non riesco a salvare la sincronizzazione");
});

test("Controlla e avvia on a saved sync goes straight to a fresh preview, never running by itself", async () => {
  backend({ list_syncs: () => [savedSync], preview_sync: () => preview(3, 0), run_sync: () => doneRun });
  render(App);

  await click(await screen.findByRole("button", { name: "Controlla e avvia Foto di Anna" }));

  expect(await screen.findByRole("heading", { name: "Anteprima: Foto di Anna" })).toBeTruthy();
  expect(calls("preview_sync")).toEqual([
    ["preview_sync", { source: "D:\\Foto", destination: "E:\\Backup", mode: "Copy" }],
  ]);
  expect(await screen.findByText("Da copiare (nuovi)")).toBeTruthy();
  expect(calls("run_sync")).toHaveLength(0);
  expect(screen.getByRole("button", { name: "No, torna all’elenco" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Indietro, cambio qualcosa" })).toBeNull();
  expect(screen.getByRole("complementary").textContent).toContain("E:\\Backup");
});

test("running a saved sync again does not save it twice and uses its name", async () => {
  backend({ list_syncs: () => [savedSync], preview_sync: () => preview(3, 0), run_sync: () => doneRun });
  render(App);
  await click(await screen.findByRole("button", { name: "Controlla e avvia Foto di Anna" }));

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  await screen.findByText(/Fatto:/);
  expect(calls("save_sync")).toHaveLength(0);
  expect(invokeMock).toHaveBeenLastCalledWith(
    "run_sync",
    expect.objectContaining({ syncName: "Foto di Anna", source: "D:\\Foto", destination: "E:\\Backup" }),
  );
});

test("No, torna all’elenco leaves a saved sync as it is and shows the list again", async () => {
  backend({ list_syncs: () => [savedSync], preview_sync: () => preview(3, 0) });
  render(App);
  await click(await screen.findByRole("button", { name: "Controlla e avvia Foto di Anna" }));

  await click(await screen.findByRole("button", { name: "No, torna all’elenco" }));

  expect(await screen.findByRole("heading", { name: "Foto di Anna" })).toBeTruthy();
  expect(calls("save_sync")).toHaveLength(0);
});

test("an unreadable syncs.json shows the empty state with Nuova sincronizzazione", async () => {
  backend({
    list_syncs: () => {
      throw "illeggibile";
    },
  });
  render(App);

  expect(await screen.findByRole("heading", { name: "Le tue sincronizzazioni" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Nuova sincronizzazione" })).toBeTruthy();
  expect(screen.queryByRole("listitem")).toBeNull();
});

test("home: a saved sync's A and B are in the schema with the caption", async () => {
  backend({ list_syncs: () => [savedSync] });
  render(App);

  await screen.findByRole("heading", { name: "Foto di Anna" });

  const schema = screen.getByRole("complementary");
  expect(schema.textContent).toContain("D:\\Foto");
  expect(schema.textContent).toContain("E:\\Backup");
  expect(schema.textContent).toContain('Premi "Controlla e avvia": prima ti mostro cosa cambierà, poi decidi tu.');
});

test("Storico lists the runs from Rust and swaps the schema for the legend", async () => {
  backend({
    list_runs: () => [
      { ...doneRun, syncName: "Seconda", outcome: "Error" },
      { ...doneRun, syncName: "Prima" },
    ],
  });
  render(App);

  await click("Storico");

  expect(invokeMock).toHaveBeenCalledWith("list_runs");
  expect(screen.getByRole("heading", { name: "Storico" })).toBeTruthy();
  expect(await screen.findByText("Seconda")).toBeTruthy();
  const aside = screen.getByRole("complementary", { name: "Come leggere lo storico" });
  expect(aside.textContent).not.toContain("Cosa succede");
  expect(within(aside).getByText("Copie fatte").previousElementSibling?.textContent).toBe("2");
  expect(within(aside).getByText("Riuscite").previousElementSibling?.textContent).toBe("1");
});

test("a failed list_runs says so instead of showing an empty history", async () => {
  backend({
    list_runs: () => {
      throw "boom";
    },
  });
  render(App);

  await click("Storico");

  expect((await screen.findByRole("alert")).textContent).toContain("Non riesco a leggere lo storico.");
  expect(screen.queryByRole("listitem")).toBeNull();
});

test("while the history loads there are no old rows, and a late older answer is ignored", async () => {
  let answerFirst!: (runs: Run[]) => void;
  const answers = [new Promise<Run[]>((resolve) => (answerFirst = resolve)), Promise.resolve([{ ...doneRun, syncName: "Aggiornata" }])];
  backend({ list_runs: () => answers.shift() });
  render(App);

  await click("Storico");
  expect(screen.getByText("Sto leggendo lo storico…")).toBeTruthy();
  await click("Sincronizzazioni");
  await click("Storico");
  expect(await screen.findByText("Aggiornata")).toBeTruthy();

  answerFirst([{ ...doneRun, syncName: "Vecchia" }]);
  await Promise.resolve();

  expect(screen.queryByText("Vecchia")).toBeNull();
  expect(screen.getByText("Aggiornata")).toBeTruthy();
});

test("closing the window during a run asks to stop the copy", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => new Promise(() => {}) });
  render(App);
  await goToPreview();
  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  const closeRequested = vi.mocked(listen).mock.calls.find(([name]) => name === "close-requested")![1];
  closeRequested({ event: "close-requested", id: 1, payload: null });

  expect(await screen.findByRole("alertdialog")).toBeTruthy();
  expect(screen.getByText("Vuoi fermare la copia? I file già copiati restano in B.")).toBeTruthy();
});
