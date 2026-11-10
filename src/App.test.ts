import { fireEvent, render, screen, within } from "@testing-library/svelte";
import { expect, test } from "vitest";
import App from "./App.svelte";
import { backend, calls, doneRun, failedRun, invokeMock, preview, savedSync } from "./testing/backend";
import type { RunReport } from "./shared/types";
import type { HistorySummary, RunList } from "./history/types";
import type { SyncSummary } from "./syncs/types";

/** What `list_runs` answers: the runs plus a summary (all zeros unless a test gives one). */
const runList = (runs: RunReport[], summary: Partial<HistorySummary> = {}): RunList => ({
  runs,
  summary: { runs: 0, ok: 0, failed: 0, cancelled: 0, filesCopied: 0, filesDeleted: 0, syncs: [], ...summary },
});

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

// Wiring: the wizard's side panel shows the schema it reports (the wizard's own tests check the values).
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

// Wiring: "Annulla" on the first step (onexit) returns to the list.
test("Annulla on the first step leaves the wizard for the list", async () => {
  backend({});
  render(App);
  await click("Nuova sincronizzazione");

  await click("Annulla");

  expect(await screen.findByRole("heading", { name: "Le tue sincronizzazioni" })).toBeTruthy();
});

// Wiring: "Controlla e avvia" on a Home card opens the wizard with that sync's folders.
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

// Wiring: "Vai allo storico" on the wizard's result (onhistory) opens the Storico.
test("Torna all’elenco and Vai allo storico leave the result", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => failedRun, list_runs: () => runList([failedRun]) });
  render(App);
  await goToPreview();
  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  await click(await screen.findByRole("button", { name: "Vai allo storico" }));

  expect(await screen.findByRole("heading", { name: "Storico" })).toBeTruthy();
  expect(invokeMock).toHaveBeenCalledWith("list_runs");
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
