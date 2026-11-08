import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { act, fireEvent, render, screen, within } from "@testing-library/svelte";
import { expect, test, vi } from "vitest";
import App from "./App.svelte";
import { run } from "./testing/fixtures";
import type { RunReport } from "./shared/types";
import type { HistorySummary, RunList } from "./history/types";
import type { Preview } from "./wizard/types";
import type { SyncSummary } from "./syncs/types";

const invokeMock = vi.mocked(invoke);

const preview = (n: number, u: number): Preview => ({
  new: { count: n, files: ["sub\\a.jpg"] },
  updated: { count: u, files: u ? ["b.pdf"] : [] },
  deleted: { count: 0, files: [] },
  bytes: 3_000_000,
  blocked: null,
  previewToken: "token-1",
  // The sentences Rust makes for these counts in Copy mode (see shared/copy_text.rs).
  caption: n + u === 0 ? "B è già aggiornato." : `Da fare: copiare ${n} file nuovi, aggiornarne ${u}.`,
  summary: [
    { text: "Ecco cosa verrà fatto: ", bold: false },
    { text: String(n), bold: true },
    { text: " file nuovi, ", bold: false },
    { text: String(u), bold: true },
    { text: " da aggiornare.", bold: false },
  ],
});

// A run as Rust sends it, texts included: the screens show them as they are.
const doneRun = run({
  new: 2,
  durationMs: 2_000,
  startedAt: "2026-11-03T12:00:00+01:00",
  logPath: "C:\\L\\logs\\x.log",
  title: "Fatto: 2 file copiati, 1 aggiornato in < 1 min",
  when: "03/11/2026 12:00",
  durationText: "< 1 min",
});

/** What `list_runs` answers: the runs plus a summary (all zeros unless a test gives one). */
const runList = (runs: RunReport[], summary: Partial<HistorySummary> = {}): RunList => ({
  runs,
  summary: { runs: 0, ok: 0, failed: 0, cancelled: 0, filesCopied: 0, filesDeleted: 0, syncs: [], ...summary },
});

const savedSync: SyncSummary = {
  id: 7,
  name: "Foto di Anna",
  source: "D:\\Foto",
  destination: "E:\\Backup",
  mode: "Copy",
  sourcePlace: { isNetwork: false, label: "Disco D:" },
  destinationPlace: { isNetwork: false, label: "Disco E:" },
  lastRun: { when: "02/11/2026 21:30", outcome: "Ok", note: null },
};

type Args = Record<string, unknown> | undefined;
type Handlers = Partial<
  Record<
    | "preview_sync"
    | "run_sync"
    | "list_syncs"
    | "save_sync"
    | "default_sync_name"
    | "check_step"
    | "list_runs"
    | "read_run_log"
    | "is_elevated",
    (args: Args) => unknown
  >
>;

/** Answers the Rust commands the app uses; by default the computer has no folders and no saved syncs. */
function backend(handlers: Handlers = {}) {
  const defaults: Record<string, (args: Args) => unknown> = {
    browse_folders: () => ({ parent: "", folders: [], error: null }),
    // What `describe_place` says for the paths these tests use (see shared/copy_text.rs).
    describe_place: (args) => {
      const labels: Record<string, string> = { "D:\\Foto": "Disco D:", "E:\\Backup": "Disco E:" };
      return { isNetwork: false, label: labels[(args as { path: string }).path] ?? "Cartella" };
    },
    list_syncs: () => ({ syncs: [], setAside: false }),
    default_sync_name: () => "Copia di Foto",
    // The words and rules of `check_step` in Rust (wizard/commands.rs and shared/path_rules.rs).
    check_step: (args) => {
      const { step, source, destination, name } = args as Record<string, string>;
      const missing: Record<string, [string, string]> = {
        source: [source, "Scegli prima la cartella da copiare."],
        destination: [destination, "Scegli prima dove mettere la copia."],
        name: [name, "Scrivi un nome per riconoscere questa sincronizzazione."],
      };
      const [value, title] = missing[step] ?? ["x", ""];
      if (!value.trim()) throw { title, advice: "" };
      if (step === "destination") {
        const plain = (path: string) => path.trim().toLowerCase().replace(/\\+$/, "");
        if (plain(source) === plain(destination))
          throw { title: "La destinazione è la stessa cartella da copiare. Scegline un’altra.", advice: "" };
        if (plain(destination).startsWith(plain(source) + "\\"))
          throw { title: "La destinazione non può stare dentro la cartella da copiare.", advice: "" };
      }
    },
    is_elevated: () => false,
    save_sync: (args) => ({ id: 7, ...args }),
  };
  invokeMock.mockImplementation(async (cmd, args) => {
    const handler = (handlers as Record<string, (args: Args) => unknown>)[cmd] ?? defaults[cmd];
    if (!handler) throw new Error(`unexpected command ${cmd}`);
    return handler(args as Args);
  });
}

const calls = (cmd: string) => invokeMock.mock.calls.filter(([c]) => c === cmd);

const click = (target: string | RegExp | HTMLElement) =>
  fireEvent.click(
    typeof target === "object" && "tagName" in target ? target : screen.getByRole("button", { name: target }),
  );
const typeInto = (label: RegExp, value: string) => fireEvent.input(screen.getByLabelText(label), { target: { value } });

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

test("shell: brand, theme switch, sections, a way to start, and the schema docked on the side", () => {
  backend({});
  render(App);

  expect(screen.getByRole("banner").textContent).toContain("ColomboSync"); // wordmark: "Colombo" + "Sync"
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
  expect(await within(schema).findByText("Disco D:")).toBeTruthy();
  expect(within(schema).getByText("Disco E:")).toBeTruthy();
  expect(screen.getByText("148", { selector: "aside b" })).toBeTruthy();
  expect(screen.getByText("3", { selector: "aside b" })).toBeTruthy();
  expect(screen.getByText("Nessun file viene cancellato in B")).toBeTruthy();
});

test("the schema caption on the preview is the one Rust sent, as it is", async () => {
  backend({
    preview_sync: () => ({ ...preview(3, 1), caption: "Da fare: copiare 3 file nuovi, aggiornarne 1." }),
  });
  render(App);

  await goToPreview();

  expect(
    await screen.findByText("Da fare: copiare 3 file nuovi, aggiornarne 1.", { selector: "aside p" }),
  ).toBeTruthy();
});

test("only the preview step fills the window: the other steps scroll with the page", async () => {
  backend({ preview_sync: () => preview(2, 1) });
  render(App);

  await click("Nuova sincronizzazione");
  expect(document.querySelector(".fill")).toBeNull(); // Da dove copio?
  await typeInto(/Percorso/, "D:\\Foto");
  await click("Avanti");
  await typeInto(/Percorso/, "E:\\Backup");
  await click("Avanti");
  await screen.findByDisplayValue("Copia di Foto");
  await click("Avanti"); // Come la chiamo? -> Opzioni
  expect(document.querySelector(".fill")).toBeNull();

  await click("Avanti"); // Anteprima
  await screen.findByText("Da copiare (nuovi)");
  expect(document.querySelector(".content.fill")).not.toBeNull();
});

test("Sì, avvia runs with the token of the preview and no counts, and says Fatto", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => doneRun });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  expect(await screen.findByText("Fatto: 2 file copiati, 1 aggiornato in < 1 min")).toBeTruthy();
  expect(invokeMock).toHaveBeenLastCalledWith("run_sync", {
    sync: { id: 7, name: "Copia di Foto", source: "D:\\Foto", destination: "E:\\Backup", mode: "Copy" },
    previewToken: "token-1",
    consent: false,
  });
});

test("when Rust refuses the run, the result shows its title and advice", async () => {
  backend({
    preview_sync: () => preview(2, 1),
    run_sync: () => {
      throw {
        title: "Non ho avviato la copia: l’anteprima non è più valida.",
        advice: "Controlla di nuovo le cartelle con l’anteprima, poi avvia.",
      };
    },
  });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  expect(
    await screen.findByRole("heading", { name: "Non ho avviato la copia: l’anteprima non è più valida." }),
  ).toBeTruthy();
  expect(screen.getByText("Controlla di nuovo le cartelle con l’anteprima, poi avvia.")).toBeTruthy();
});

test("when Tauri itself rejects with a plain string, the result says the fixed sentence, never undefined", async () => {
  backend({
    preview_sync: () => preview(2, 1),
    run_sync: () => {
      throw "invalid args `consent` for command `run_sync`";
    },
  });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  expect(await screen.findByRole("heading", { name: "Qualcosa è andato storto dentro l’app." })).toBeTruthy();
  expect(screen.getByText("Riprova. Se succede ancora, chiudi e riapri ColomboSync.")).toBeTruthy();
  expect(document.body.textContent).not.toContain("undefined");
});

test("while the copy runs the UI says Sto copiando…", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => new Promise(() => {}) });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  expect(screen.getByRole("heading", { name: /^Sto copiando/ })).toBeTruthy();
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
      throw { title: "robocopy ha restituito il codice 16.", advice: "" };
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
  await screen.findByRole("heading", { name: "Come la chiamo?" });
  await click("Avanti");
  await screen.findByRole("heading", { name: "Opzioni" });
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

test("the new sync is saved when the run starts, under the name typed, and the run is of the saved sync", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => doneRun });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  await screen.findByText(/Fatto:/);
  expect(calls("save_sync")).toEqual([
    ["save_sync", { name: "Copia di Foto", source: "D:\\Foto", destination: "E:\\Backup", mode: "Copy" }],
  ]);
  expect(invokeMock).toHaveBeenLastCalledWith("run_sync", {
    sync: { id: 7, name: "Copia di Foto", source: "D:\\Foto", destination: "E:\\Backup", mode: "Copy" },
    previewToken: "token-1",
    consent: false,
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
      throw { title: "Non riesco a salvare la sincronizzazione: disco pieno.", advice: "" };
    },
  });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "No, salva e basta" }));

  expect((await screen.findByRole("alert")).textContent).toContain("Non riesco a salvare la sincronizzazione");
});

test("Controlla e avvia on a saved sync goes straight to a fresh preview, never running by itself", async () => {
  backend({
    list_syncs: () => ({ syncs: [savedSync], setAside: false }),
    preview_sync: () => preview(3, 0),
    run_sync: () => doneRun,
  });
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

test("running a saved sync again does not save it twice and runs that sync by its id", async () => {
  backend({
    list_syncs: () => ({ syncs: [savedSync], setAside: false }),
    preview_sync: () => preview(3, 0),
    run_sync: () => doneRun,
  });
  render(App);
  await click(await screen.findByRole("button", { name: "Controlla e avvia Foto di Anna" }));

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  await screen.findByText(/Fatto:/);
  expect(calls("save_sync")).toHaveLength(0);
  expect(invokeMock).toHaveBeenLastCalledWith(
    "run_sync",
    expect.objectContaining({
      sync: expect.objectContaining({ id: 7, name: "Foto di Anna", source: "D:\\Foto", destination: "E:\\Backup" }),
    }),
  );
});

test("No, torna all’elenco leaves a saved sync as it is and shows the list again", async () => {
  backend({ list_syncs: () => ({ syncs: [savedSync], setAside: false }), preview_sync: () => preview(3, 0) });
  render(App);
  await click(await screen.findByRole("button", { name: "Controlla e avvia Foto di Anna" }));

  await click(await screen.findByRole("button", { name: "No, torna all’elenco" }));

  expect(await screen.findByRole("heading", { name: "Foto di Anna" })).toBeTruthy();
  expect(calls("save_sync")).toHaveLength(0);
});

test("Controlla e avvia on a saved Mirror sync still shows the red group and asks for the consent before running", async () => {
  const mirrorSync: SyncSummary = { ...savedSync, mode: "Mirror" };
  backend({
    list_syncs: () => ({ syncs: [mirrorSync], setAside: false }),
    preview_sync: () => ({ ...preview(20, 1), deleted: { count: 3, files: ["vecchio.txt"] } }),
    run_sync: () => doneRun,
  });
  render(App);
  expect(
    ((await screen.findByRole("heading", { name: "Foto di Anna" })).closest("li") as HTMLElement).textContent,
  ).toContain("Specchio");

  await click(screen.getByRole("button", { name: "Controlla e avvia Foto di Anna" }));

  expect(calls("preview_sync")).toEqual([
    ["preview_sync", { source: "D:\\Foto", destination: "E:\\Backup", mode: "Mirror" }],
  ]);
  const start = (await screen.findByRole("button", { name: "Sì, avvia" })) as HTMLButtonElement;
  expect(screen.getByText("Da cancellare in B")).toBeTruthy();
  expect(calls("run_sync")).toHaveLength(0);
  expect(start.disabled).toBe(true);

  await fireEvent.click(screen.getByLabelText("Ho capito: 3 file verranno cancellati da B"));
  await click(start);

  await screen.findByText(/Fatto:/);
  expect(invokeMock).toHaveBeenLastCalledWith(
    "run_sync",
    expect.objectContaining({
      sync: expect.objectContaining({ id: 7, mode: "Mirror" }),
      previewToken: "token-1",
      consent: true,
    }),
  );
  expect(calls("save_sync")).toHaveLength(0);
});

test("a syncs.json that cannot be read nor set aside says so, shows no list and does not let a new sync be started over it", async () => {
  backend({
    list_syncs: () => {
      throw { title: "Non riesco a leggere le sincronizzazioni salvate.", advice: "" };
    },
  });
  render(App);

  expect((await screen.findByRole("alert")).textContent).toContain("Non riesco a leggere le sincronizzazioni salvate.");
  expect((screen.getByRole("button", { name: "Nuova sincronizzazione" }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.queryByRole("listitem")).toBeNull();
});

test("clicking No, salva e basta twice quickly saves once", async () => {
  backend({ preview_sync: () => preview(2, 1) });
  render(App);
  await goToPreview();
  const no = await screen.findByRole("button", { name: "No, salva e basta" });

  fireEvent.click(no);
  fireEvent.click(no);

  expect(await screen.findByRole("heading", { name: "Le tue sincronizzazioni" })).toBeTruthy();
  expect(calls("save_sync")).toHaveLength(1);
});

test("clicking Sì, avvia twice quickly saves and runs once", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => doneRun });
  render(App);
  await goToPreview();
  const yes = await screen.findByRole("button", { name: "Sì, avvia" });

  fireEvent.click(yes);
  fireEvent.click(yes);

  await screen.findByText(/Fatto:/);
  expect(calls("save_sync")).toHaveLength(1);
  expect(calls("run_sync")).toHaveLength(1);
});

/** Lets a test answer each `default_sync_name` request when it chooses. */
function slowSuggestions() {
  const asked: { source: string; answer: (name: string) => void }[] = [];
  const base = invokeMock.getMockImplementation()!;
  invokeMock.mockImplementation(async (cmd, args) =>
    cmd === "default_sync_name"
      ? new Promise<string>((answer) => asked.push({ source: (args as { source: string }).source, answer }))
      : base(cmd, args),
  );
  return asked;
}

test("while the suggested name is on its way Avanti waits, and no empty-name error shows when it arrives", async () => {
  backend();
  const asked = slowSuggestions();
  render(App);
  await click("Nuova sincronizzazione");
  await typeInto(/Percorso/, "D:\\Foto");
  await click("Avanti");
  await typeInto(/Percorso/, "E:\\Backup");
  await click("Avanti");
  await screen.findByRole("heading", { name: "Come la chiamo?" });

  expect((screen.getByRole("button", { name: "Avanti" }) as HTMLButtonElement).disabled).toBe(true);
  await act(() => asked[0].answer("Copia di Foto"));

  expect((screen.getByRole("button", { name: "Avanti" }) as HTMLButtonElement).disabled).toBe(false);
  expect(screen.queryByRole("alert")).toBeNull();
  expect((screen.getByLabelText("Nome della sincronizzazione") as HTMLInputElement).value).toBe("Copia di Foto");
});

test("an older suggested name arriving late does not replace the newer one", async () => {
  backend();
  const asked = slowSuggestions();
  render(App);
  await click("Nuova sincronizzazione");
  await typeInto(/Percorso/, "D:\\Foto");
  await click("Avanti");
  await typeInto(/Percorso/, "E:\\Backup");
  await click("Avanti"); // asks for D:\Foto
  await screen.findByRole("heading", { name: "Come la chiamo?" });
  await click("Indietro");
  await click("Indietro");
  await typeInto(/Percorso/, "D:\\Altro");
  await click("Avanti");
  await screen.findByRole("heading", { name: "Dove copio?" });
  await click("Avanti"); // asks for D:\Altro
  await screen.findByRole("heading", { name: "Come la chiamo?" });

  await act(() => asked[1].answer("Copia di Altro"));
  await act(() => asked[0].answer("Copia di Foto"));

  expect((screen.getByLabelText("Nome della sincronizzazione") as HTMLInputElement).value).toBe("Copia di Altro");
});

test("home: a saved sync's A and B are in the schema with the caption", async () => {
  backend({ list_syncs: () => ({ syncs: [savedSync], setAside: false }) });
  render(App);

  await screen.findByRole("heading", { name: "Foto di Anna" });

  const schema = screen.getByRole("complementary");
  expect(schema.textContent).toContain("D:\\Foto");
  expect(schema.textContent).toContain("E:\\Backup");
  expect(schema.textContent).toContain('Premi "Controlla e avvia": prima ti mostro cosa cambierà, poi decidi tu.');
});

test("Storico lists the runs from Rust and swaps the schema for the summary panel", async () => {
  backend({
    list_runs: () =>
      runList(
        [
          { ...doneRun, syncName: "Seconda", outcome: "Error" },
          { ...doneRun, syncName: "Prima" },
        ],
        {
          runs: 2,
          ok: 1,
          failed: 1,
          filesCopied: 3,
          syncs: [
            { id: 7, name: "Foto di Anna", lastSuccessText: "Ultima copia riuscita: 03/11/2026 12:00", warning: null },
          ],
        },
      ),
  });
  render(App);

  await click("Storico");

  expect(invokeMock).toHaveBeenCalledWith("list_runs");
  expect(calls("history_summary")).toHaveLength(0);
  expect(screen.getByRole("heading", { name: "Storico" })).toBeTruthy();
  expect(await screen.findByText("Seconda", { selector: "strong" })).toBeTruthy();
  const aside = screen.getByRole("complementary", { name: "Riepilogo dello storico" });
  expect(aside.textContent).not.toContain("Cosa succede");
  expect(within(aside).getByText("Copie fatte").previousElementSibling?.textContent).toBe("2");
  expect(within(aside).getByText("Con errore").previousElementSibling?.textContent).toBe("1");
  expect(within(aside).getByText("Foto di Anna")).toBeTruthy();
});

test("a failed list_runs says so instead of showing an empty history", async () => {
  backend({
    list_runs: () => {
      throw { title: "Non riesco a leggere lo storico.", advice: "" };
    },
  });
  render(App);

  await click("Storico");

  expect((await screen.findByRole("alert")).textContent).toContain("Non riesco a leggere lo storico.");
  expect(screen.queryByRole("listitem")).toBeNull();
});

test("while the history loads there are no old rows, and a late older answer is ignored", async () => {
  let answerFirst!: (list: RunList) => void;
  const answers = [
    new Promise<RunList>((resolve) => (answerFirst = resolve)),
    Promise.resolve(runList([{ ...doneRun, syncName: "Aggiornata" }])),
  ];
  backend({ list_runs: () => answers.shift() });
  render(App);

  await click("Storico");
  expect(screen.getByText("Sto leggendo lo storico…")).toBeTruthy();
  await click("Sincronizzazioni");
  await click("Storico");
  expect(await screen.findByText("Aggiornata", { selector: "strong" })).toBeTruthy();

  answerFirst(runList([{ ...doneRun, syncName: "Vecchia" }]));
  await Promise.resolve();

  expect(screen.queryByText("Vecchia", { selector: "strong" })).toBeNull();
  expect(screen.getByText("Aggiornata", { selector: "strong" })).toBeTruthy();
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

const toOpzioni = async () => {
  await click("Nuova sincronizzazione");
  await typeInto(/Percorso/, String.raw`D:\Foto`);
  await click("Avanti");
  await typeInto(/Percorso/, String.raw`E:\Backup`);
  await click("Avanti");
  await screen.findByDisplayValue("Copia di Foto");
  await click("Avanti"); // Come la chiamo?
  await screen.findByRole("heading", { name: "Opzioni" });
};
const mirrorSwitch = () => screen.getByRole("switch", { name: "Rendi B identico ad A (specchio)" }) as HTMLInputElement;

test("Opzioni: advanced options are collapsed and mirror is off", async () => {
  backend({});
  render(App);
  await toOpzioni();

  expect(screen.getByText("Opzioni avanzate").closest("details")?.open).toBe(false);
  expect(mirrorSwitch().checked).toBe(false);
  expect(screen.queryByText("Attenzione: questa opzione cancella dei file")).toBeNull();
  expect(screen.getByText("Nessun file viene cancellato in B")).toBeTruthy();
});

test("turning on the mirror switch shows the red warning at once and the schema becomes Specchio", async () => {
  backend({});
  render(App);
  await toOpzioni();

  await fireEvent.click(mirrorSwitch());

  expect(screen.getByRole("alert").textContent).toContain("Attenzione: questa opzione cancella dei file");
  expect(screen.getByRole("alert").textContent).toContain(
    "Prima di partire vedrai l’elenco dei file che verranno cancellati.",
  );
  const schema = screen.getByRole("complementary");
  expect(schema.textContent).toContain("Specchio");
  expect(schema.textContent).toContain("Eliminati da B");
  expect(schema.textContent).not.toContain("Nessun file viene cancellato in B");

  await fireEvent.click(mirrorSwitch());
  expect(screen.queryByText("Attenzione: questa opzione cancella dei file")).toBeNull();
  expect(screen.getByRole("complementary").textContent).not.toContain("Eliminati da B");
});

test("mirror on: the preview runs in Mirror mode, lists what is deleted and the schema counts it", async () => {
  backend({
    preview_sync: () => ({ ...preview(20, 1), deleted: { count: 12, files: ["vecchio.txt"] } }),
    run_sync: () => ({
      ...doneRun,
      mode: "Mirror",
      deleted: 12,
      title: "Fatto: 2 file copiati, 1 aggiornato, 12 cancellati da B in < 1 min",
    }),
  });
  render(App);
  await toOpzioni();
  await fireEvent.click(mirrorSwitch());
  await click("Avanti");

  expect(invokeMock).toHaveBeenCalledWith("preview_sync", {
    source: String.raw`D:\Foto`,
    destination: String.raw`E:\Backup`,
    mode: "Mirror",
  });
  expect((await screen.findByText("Da cancellare in B")).nextElementSibling?.textContent).toBe("12");
  expect(screen.getByText("12", { selector: "aside b" })).toBeTruthy();
  const start = screen.getByRole("button", { name: "Sì, avvia" }) as HTMLButtonElement;
  expect(start.disabled).toBe(true);

  await fireEvent.click(screen.getByLabelText("Ho capito: 12 file verranno cancellati da B"));
  await click(start);

  expect(await screen.findByText(/Fatto: 2 file copiati, 1 aggiornato, 12 cancellati da B/)).toBeTruthy();
  expect(invokeMock).toHaveBeenLastCalledWith("run_sync", {
    sync: {
      id: 7,
      name: "Copia di Foto",
      source: String.raw`D:\Foto`,
      destination: String.raw`E:\Backup`,
      mode: "Mirror",
    },
    previewToken: "token-1",
    consent: true,
  });
});

test("consent is per run: going back and opening the preview again unchecks it", async () => {
  backend({ preview_sync: () => ({ ...preview(20, 1), deleted: { count: 3, files: ["x"] } }) });
  render(App);
  await toOpzioni();
  await fireEvent.click(mirrorSwitch());
  await click("Avanti");
  await fireEvent.click(await screen.findByLabelText(/Ho capito/));

  await click("Indietro, cambio qualcosa");
  await click("Avanti");

  expect(((await screen.findByLabelText(/Ho capito/)) as HTMLInputElement).checked).toBe(false);
});

test("MirrorGuard: a blocked mirror preview shows Rust's reason, in the schema too, and offers no Sì, avvia", async () => {
  const reason = "Motivo deciso da Rust: non si parte.";
  backend({ preview_sync: () => ({ ...preview(0, 0), blocked: reason, caption: reason }) });
  render(App);
  await toOpzioni();
  await fireEvent.click(mirrorSwitch());
  await click("Avanti");

  expect((await screen.findAllByText(reason)).length).toBe(2); // the warning and the schema caption
  expect(screen.getByRole("complementary").textContent).toContain(reason);
  expect(screen.getByRole("complementary").textContent).not.toContain("B è già aggiornato");
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
});

test("a blocked preview that still counts files does not say 'Da fare' in the schema", async () => {
  const reason = "Motivo deciso da Rust: non si parte.";
  backend({ preview_sync: () => ({ ...preview(2, 1), blocked: reason, caption: reason }) });
  render(App);
  await toOpzioni();
  await fireEvent.click(mirrorSwitch());
  await click("Avanti");

  await screen.findAllByText(reason);
  expect(screen.getByRole("complementary").textContent).not.toContain("Da fare");
});

test("Dove: Avanti is blocked with the message from Rust when B is A or inside A", async () => {
  backend();
  render(App);
  await click("Nuova sincronizzazione");
  await typeInto(/Percorso/, String.raw`D:\Foto`);
  await click("Avanti");
  await typeInto(/Percorso/, "d:\\foto\\");

  await click("Avanti");

  expect(invokeMock).toHaveBeenCalledWith("check_step", {
    step: "destination",
    source: String.raw`D:\Foto`,
    destination: "d:\\foto\\",
    name: "",
  });
  expect(screen.getByRole("alert").textContent).toContain(
    "La destinazione è la stessa cartella da copiare. Scegline un’altra.",
  );
  expect(screen.getByRole("heading", { name: "Dove copio?" })).toBeTruthy();

  await typeInto(/Percorso/, String.raw`D:\Foto\Backup`);
  await click("Avanti");

  expect(screen.getByRole("alert").textContent).toContain(
    "La destinazione non può stare dentro la cartella da copiare.",
  );
  expect(screen.getByRole("heading", { name: "Dove copio?" })).toBeTruthy();
});

test("Dove: if the paths change while they are being checked, the new ones are checked before moving on", async () => {
  let release: () => void = () => {};
  let destinationChecks = 0;
  backend({
    check_step: (args) => {
      if ((args as { step: string }).step !== "destination") return;
      destinationChecks++;
      if (destinationChecks === 1) return new Promise<void>((resolve) => (release = resolve));
      throw { title: "La destinazione non può stare dentro la cartella da copiare.", advice: "" };
    },
  });
  render(App);
  await click("Nuova sincronizzazione");
  await typeInto(/Percorso/, String.raw`D:\Foto`);
  await click("Avanti");
  await typeInto(/Percorso/, String.raw`E:\Backup`);
  const first = click("Avanti");

  await typeInto(/Percorso/, String.raw`D:\Foto\Backup`); // changed while the first check is pending
  release();
  await first;
  await vi.waitFor(() => expect(destinationChecks).toBe(2));

  expect(invokeMock).toHaveBeenLastCalledWith("check_step", {
    step: "destination",
    source: String.raw`D:\Foto`,
    destination: String.raw`D:\Foto\Backup`,
    name: "",
  });
  await vi.waitFor(() => expect(screen.getByRole("alert").textContent).toContain("non può stare dentro"));
  expect(screen.getByRole("heading", { name: "Dove copio?" })).toBeTruthy();
});

test("Nome: a refusal that arrives after Indietro is dropped, not shown on Dove", async () => {
  let refuse: (e: unknown) => void = () => {};
  backend({
    check_step: (args) => {
      if ((args as { step: string }).step !== "name") return;
      return new Promise<void>((_, reject) => (refuse = reject));
    },
  });
  render(App);
  await click("Nuova sincronizzazione");
  await typeInto(/Percorso/, String.raw`D:\Foto`);
  await click("Avanti");
  await typeInto(/Percorso/, String.raw`E:\Backup`);
  await click("Avanti");
  await screen.findByDisplayValue("Copia di Foto");
  await typeInto(/Nome della sincronizzazione/, "");
  const pending = click("Avanti");
  await vi.waitFor(() => expect(calls("check_step").length).toBe(3));

  await click("Indietro");
  refuse({ title: "Scrivi un nome per riconoscere questa sincronizzazione.", advice: "" });
  await pending;
  await act();

  expect(screen.getByRole("heading", { name: "Dove copio?" })).toBeTruthy();
  expect(screen.queryByRole("alert")).toBeNull();
});

test("Nome: a refusal for a name that was edited meanwhile is not shown, the new name is checked", async () => {
  let refuse: (e: unknown) => void = () => {};
  let nameChecks = 0;
  backend({
    check_step: (args) => {
      if ((args as { step: string }).step !== "name") return;
      if (++nameChecks === 1) return new Promise<void>((_, reject) => (refuse = reject));
    },
  });
  render(App);
  await click("Nuova sincronizzazione");
  await typeInto(/Percorso/, String.raw`D:\Foto`);
  await click("Avanti");
  await typeInto(/Percorso/, String.raw`E:\Backup`);
  await click("Avanti");
  await screen.findByDisplayValue("Copia di Foto");
  await typeInto(/Nome della sincronizzazione/, "");
  const pending = click("Avanti");
  await vi.waitFor(() => expect(nameChecks).toBe(1));

  await typeInto(/Nome della sincronizzazione/, "Foto");
  refuse({ title: "Scrivi un nome per riconoscere questa sincronizzazione.", advice: "" });
  await pending;

  await vi.waitFor(() => expect(screen.getByRole("heading", { name: "Opzioni" })).toBeTruthy());
  expect(nameChecks).toBe(2);
  expect(screen.queryByRole("alert")).toBeNull();
});

test("Opzioni: Avanti asks Rust nothing, it goes straight to the preview", async () => {
  backend({ preview_sync: () => preview(2, 1) });
  render(App);
  await goToPreview();

  expect(calls("check_step").map(([, args]) => (args as { step: string }).step)).toEqual([
    "source",
    "destination",
    "name",
  ]);
});

// What Rust sends (error_messages): the sentence that says what happened, and what to do.
const unreachable = {
  title: String.raw`Non completato: la cartella di rete \\NAS non è raggiungibile.`,
  advice: "Controlla la connessione e riprova.",
};
const failedRun: RunReport = {
  ...doneRun,
  outcome: "Error",
  explanation: unreachable,
  partial: 0,
  title: unreachable.title,
  caption: "Non completato.",
  resultNote: "Nessun file è stato copiato.",
  partialText: "0 file copiati (parziale)",
};

test("a run that fails says why in plain Italian, with Riprova and the log link", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => failedRun });
  render(App);
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  expect(await screen.findByRole("heading", { name: unreachable.title })).toBeTruthy();
  expect(screen.getByText(unreachable.advice)).toBeTruthy();
  expect(screen.getByRole("button", { name: "Riprova" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Apri il registro" })).toBeTruthy();
});

test("Riprova opens a fresh preview of the same saved sync and never runs by itself", async () => {
  let previews = 0;
  backend({
    preview_sync: () => ({ ...preview(2, 1), previewToken: `token-${++previews}` }),
    run_sync: () => failedRun,
  });
  render(App);
  await goToPreview();
  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  await click(await screen.findByRole("button", { name: "Riprova" }));

  expect(await screen.findByRole("heading", { name: "Anteprima: Copia di Foto" })).toBeTruthy();
  expect(calls("preview_sync")).toHaveLength(2);
  expect(calls("preview_sync")[1]).toEqual(calls("preview_sync")[0]);
  expect(calls("run_sync")).toHaveLength(1);
  expect(await screen.findByText("Da copiare (nuovi)")).toBeTruthy();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  await screen.findByRole("button", { name: "Riprova" });
  expect(calls("save_sync")).toHaveLength(1);
  expect(calls("run_sync")).toHaveLength(2);
  expect(calls("run_sync")[0][1]).toEqual(expect.objectContaining({ previewToken: "token-1" }));
  expect(calls("run_sync")[1][1]).toEqual(
    expect.objectContaining({ sync: expect.objectContaining({ id: 7 }), previewToken: "token-2" }),
  );
});

test("Torna all’elenco and Vai allo storico leave the result", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => failedRun, list_runs: () => runList([failedRun]) });
  render(App);
  await goToPreview();
  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  await click(await screen.findByRole("button", { name: "Vai allo storico" }));

  expect(await screen.findByRole("heading", { name: "Storico" })).toBeTruthy();
  expect(invokeMock).toHaveBeenCalledWith("list_runs");
});

test("Riprova after a stopped run shows the preview again", async () => {
  backend({
    preview_sync: () => preview(2, 1),
    run_sync: () => ({
      ...doneRun,
      outcome: "Cancelled",
      partial: 1,
      title: "Annullato: ho fermato la copia.",
      caption: "Annullato.",
      resultNote: "1 file copiato (parziale). I file già copiati restano in B.",
      partialText: "1 file copiato (parziale)",
    }),
  });
  render(App);
  await goToPreview();
  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  await click(await screen.findByRole("button", { name: "Riprova" }));

  expect(await screen.findByText("Da copiare (nuovi)")).toBeTruthy();
});

test("running as administrator shows the banner about network drives", async () => {
  backend({ is_elevated: () => true });
  render(App);

  expect(
    await screen.findByText(
      "Le unità di rete potrebbero non essere visibili: apri ColomboSync senza 'Esegui come amministratore'.",
    ),
  ).toBeTruthy();
});

test("a normal user, or an unknown answer, sees no banner", async () => {
  backend({ is_elevated: () => false });
  const { unmount } = render(App);
  await screen.findByRole("button", { name: "Nuova sincronizzazione" });
  expect(calls("is_elevated")).toHaveLength(1);
  expect(screen.queryByText(/Esegui come amministratore/)).toBeNull();
  unmount();

  backend({
    is_elevated: () => {
      throw "boom";
    },
  });
  render(App);
  await screen.findByRole("button", { name: "Nuova sincronizzazione" });
  expect(screen.queryByText(/Esegui come amministratore/)).toBeNull();
});

test("a preview that fails says what happened and what to do, and Riprova checks again", async () => {
  const failure = {
    title: String.raw`Non riesco a controllare le cartelle: la cartella di rete \NAS non è raggiungibile.`,
    advice: "Controlla la connessione e riprova.",
  };
  let attempts = 0;
  backend({
    preview_sync: () => {
      if (++attempts === 1) throw failure;
      return preview(2, 1);
    },
  });
  render(App);
  await goToPreview();

  expect((await screen.findByRole("alert")).textContent).toContain(failure.title);
  expect(screen.getByText(failure.advice)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();

  await click("Riprova");

  expect(await screen.findByRole("button", { name: "Sì, avvia" })).toBeTruthy();
  expect(calls("preview_sync")).toHaveLength(2);
  expect(screen.queryByRole("alert")).toBeNull();
});
