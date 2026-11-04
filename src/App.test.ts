import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import { beforeEach, expect, test, vi } from "vitest";
import App from "./App.svelte";
import type { Preview, Run } from "./lib/types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));

const invokeMock = vi.mocked(invoke);

const preview = (n: number, u: number): Preview => ({
  new: { count: n, files: ["sub\\a.jpg"] },
  updated: { count: u, files: u ? ["b.pdf"] : [] },
  deleted: { count: 0, files: [] },
  bytes: 3_000_000,
  blocked: false,
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
function backend(handlers: {
  preview_sync?: () => unknown;
  run_sync?: () => unknown;
  check_destination?: () => unknown;
  list_runs?: () => unknown;
  read_run_log?: () => unknown;
}) {
  invokeMock.mockImplementation(async (cmd) => {
    if (cmd === "browse_folders") return { parent: "", folders: [], error: null };
    if (cmd === "check_destination" && !handlers.check_destination) return undefined; // B is fine
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

const toOpzioni = async () => {
  await click("+ Nuova sincronizzazione");
  await typeInto(/Percorso/, String.raw`D:\Foto`);
  await click("Avanti");
  await typeInto(/Percorso/, String.raw`E:\Backup`);
  await click("Avanti");
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
  expect(screen.getByRole("alert").textContent).toContain("Prima di partire vedrai l’elenco dei file che verranno cancellati.");
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
    run_sync: () => ({ ...doneRun, mode: "Mirror", deleted: 12 }),
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
    source: String.raw`D:\Foto`,
    destination: String.raw`E:\Backup`,
    mode: "Mirror",
    counts: { new: 20, updated: 1, deleted: 12 },
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

test("MirrorGuard: a blocked mirror preview offers no Sì, avvia", async () => {
  backend({ preview_sync: () => ({ ...preview(0, 0), blocked: true }) });
  render(App);
  await toOpzioni();
  await fireEvent.click(mirrorSwitch());
  await click("Avanti");

  expect((await screen.findAllByText(/A sembra vuota o scollegata/)).length).toBeGreaterThan(0);
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
});

test("Dove: Avanti is blocked with the message from Rust when B is A or inside A", async () => {
  let message = "La destinazione è la stessa cartella da copiare. Scegline un’altra.";
  backend({
    check_destination: () => {
      throw message;
    },
  });
  render(App);
  await click("+ Nuova sincronizzazione");
  await typeInto(/Percorso/, String.raw`D:\Foto`);
  await click("Avanti");
  await typeInto(/Percorso/, "d:\\foto\\");

  await click("Avanti");

  expect(invokeMock).toHaveBeenCalledWith("check_destination", {
    source: String.raw`D:\Foto`,
    destination: "d:\\foto\\",
  });
  expect(screen.getByRole("alert").textContent).toContain(message);
  expect(screen.getByRole("heading", { name: "Dove copio?" })).toBeTruthy();

  message = "La destinazione non può stare dentro la cartella da copiare.";
  await typeInto(/Percorso/, String.raw`D:\Foto\Backup`);
  await click("Avanti");

  expect(screen.getByRole("alert").textContent).toContain(message);
  expect(screen.getByRole("heading", { name: "Dove copio?" })).toBeTruthy();
});
