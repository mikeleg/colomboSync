import { listen } from "@tauri-apps/api/event";
import { act, fireEvent, render, screen } from "@testing-library/svelte";
import { expect, test, vi } from "vitest";
import Wizard from "./Wizard.svelte";
import { backend, calls, doneRun, failedRun, invokeMock, preview, savedSync, unreachable } from "../testing/backend";
import type { Sync } from "../shared/types";

/** The Wizard as App renders it: a new sync, or the saved one `sync` to run again. */
function renderWizard(sync?: Sync) {
  const onexit = vi.fn();
  const { component } = render(Wizard, { onexit, onhistory: vi.fn(), sync });
  return { component, onexit };
}

const click = (target: string | RegExp | HTMLElement) =>
  fireEvent.click(
    typeof target === "object" && "tagName" in target ? target : screen.getByRole("button", { name: target }),
  );
const typeInto = (label: RegExp, value: string) => fireEvent.input(screen.getByLabelText(label), { target: { value } });

async function goToPreview() {
  await typeInto(/Percorso/, "D:\\Foto");
  await click("Avanti");
  await typeInto(/Percorso/, "E:\\Backup");
  await click("Avanti");
  await screen.findByDisplayValue("Copia di Foto");
  await click("Avanti"); // Come la chiamo?
  await click("Avanti"); // Opzioni
}

test("the schema caption on the preview is the one Rust sent, as it is", async () => {
  backend({
    preview_sync: () => ({ ...preview(3, 1), caption: "Da fare: copiare 3 file nuovi, aggiornarne 1." }),
  });
  const { component } = renderWizard();

  await goToPreview();

  await vi.waitFor(() => expect(component.getSchema().caption).toBe("Da fare: copiare 3 file nuovi, aggiornarne 1."));
});

test("only the preview step fills the window: the other steps scroll with the page", async () => {
  backend({ preview_sync: () => preview(2, 1) });
  renderWizard();

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
  renderWizard();
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
  renderWizard();
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
  renderWizard();
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  expect(await screen.findByRole("heading", { name: "Qualcosa è andato storto dentro l’app." })).toBeTruthy();
  expect(screen.getByText("Riprova. Se succede ancora, chiudi e riapri ColomboSync.")).toBeTruthy();
  expect(document.body.textContent).not.toContain("undefined");
});

test("while the copy runs the UI says Sto copiando…", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => new Promise(() => {}) });
  renderWizard();
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  expect(screen.getByRole("heading", { name: /^Sto copiando/ })).toBeTruthy();
});

test("nothing to do: B è già aggiornato, an Indietro link and no Sì, avvia", async () => {
  backend({ preview_sync: () => preview(0, 0) });
  renderWizard();

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
  renderWizard();

  await goToPreview();

  expect((await screen.findByRole("alert")).textContent).toContain("robocopy ha restituito il codice 16.");
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
  expect(screen.getByRole("button", { name: "Indietro" })).toBeTruthy();
});

test("an empty folder blocks Avanti with a message and opens the manual input", async () => {
  backend({});
  renderWizard();

  await click("Avanti");

  expect(screen.getByRole("alert").textContent).toContain("Scegli prima la cartella da copiare.");
  expect(screen.getByText("Preferisci scrivere il percorso a mano?").closest("details")?.open).toBe(true);
});

test("Enter in the manual input behaves as Avanti", async () => {
  backend({});
  renderWizard();
  await typeInto(/Percorso/, "D:\\Foto");

  await fireEvent.keyDown(screen.getByLabelText(/Percorso/), { key: "Enter" });

  expect(screen.getByRole("heading", { name: "Dove copio?" })).toBeTruthy();
});

test("Indietro, cambio qualcosa goes back to Opzioni keeping what was typed; No, salva e basta returns to the start", async () => {
  backend({ preview_sync: () => preview(1, 0) });
  const { onexit } = renderWizard();
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

  await vi.waitFor(() => expect(onexit).toHaveBeenCalledTimes(1));
});

test("Come la chiamo? sits before Opzioni, is prefilled from A and an empty name blocks Avanti", async () => {
  backend();
  renderWizard();
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
  renderWizard();
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
  renderWizard();
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
  const { onexit } = renderWizard();
  await goToPreview();

  await click(await screen.findByRole("button", { name: "No, salva e basta" }));

  await vi.waitFor(() => expect(onexit).toHaveBeenCalledTimes(1));
  expect(calls("save_sync")).toHaveLength(1);
  expect(calls("run_sync")).toHaveLength(0);
});

test("B è già aggiornato: Salva e chiudi saves the new sync and returns to the list", async () => {
  backend({ preview_sync: () => preview(0, 0) });
  const { onexit } = renderWizard();
  await goToPreview();

  await click(await screen.findByRole("button", { name: "Salva e chiudi" }));

  await vi.waitFor(() => expect(onexit).toHaveBeenCalledTimes(1));
  expect(calls("save_sync")).toHaveLength(1);
});

test("a sync that cannot be saved says so and does not leave the preview", async () => {
  backend({
    preview_sync: () => preview(2, 1),
    save_sync: () => {
      throw { title: "Non riesco a salvare la sincronizzazione: disco pieno.", advice: "" };
    },
  });
  renderWizard();
  await goToPreview();

  await click(await screen.findByRole("button", { name: "No, salva e basta" }));

  expect((await screen.findByRole("alert")).textContent).toContain("Non riesco a salvare la sincronizzazione");
});

test("running a saved sync again does not save it twice and runs that sync by its id", async () => {
  backend({
    preview_sync: () => preview(3, 0),
    run_sync: () => doneRun,
  });
  renderWizard(savedSync);

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
  backend({ preview_sync: () => preview(3, 0) });
  const { onexit } = renderWizard(savedSync);

  await click(await screen.findByRole("button", { name: "No, torna all’elenco" }));

  await vi.waitFor(() => expect(onexit).toHaveBeenCalledTimes(1));
  expect(calls("save_sync")).toHaveLength(0);
});

test("clicking No, salva e basta twice quickly saves once", async () => {
  backend({ preview_sync: () => preview(2, 1) });
  const { onexit } = renderWizard();
  await goToPreview();
  const no = await screen.findByRole("button", { name: "No, salva e basta" });

  fireEvent.click(no);
  fireEvent.click(no);

  await vi.waitFor(() => expect(onexit).toHaveBeenCalledTimes(1));
  expect(calls("save_sync")).toHaveLength(1);
});

test("clicking Sì, avvia twice quickly saves and runs once", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => doneRun });
  renderWizard();
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
  renderWizard();
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
  renderWizard();
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

test("closing the window during a run asks to stop the copy", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => new Promise(() => {}) });
  renderWizard();
  await goToPreview();
  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  const closeRequested = vi.mocked(listen).mock.calls.find(([name]) => name === "close-requested")![1];
  closeRequested({ event: "close-requested", id: 1, payload: null });

  expect(await screen.findByRole("alertdialog")).toBeTruthy();
  expect(screen.getByText("Vuoi fermare la copia? I file già copiati restano in B.")).toBeTruthy();
});

const toOpzioni = async () => {
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
  const { component } = renderWizard();
  await toOpzioni();

  expect(screen.getByText("Opzioni avanzate").closest("details")?.open).toBe(false);
  expect(mirrorSwitch().checked).toBe(false);
  expect(screen.queryByText("Attenzione: questa opzione cancella dei file")).toBeNull();
  expect(component.getSchema().mirror).toBe(false);
});

test("turning on the mirror switch shows the red warning at once and the schema becomes Specchio", async () => {
  backend({});
  const { component } = renderWizard();
  await toOpzioni();

  await fireEvent.click(mirrorSwitch());

  expect(screen.getByRole("alert").textContent).toContain("Attenzione: questa opzione cancella dei file");
  expect(screen.getByRole("alert").textContent).toContain(
    "Prima di partire vedrai l’elenco dei file che verranno cancellati.",
  );
  expect(component.getSchema().mirror).toBe(true);
  expect(component.getSchema().warnCaption).toBe(true);

  await fireEvent.click(mirrorSwitch());
  expect(screen.queryByText("Attenzione: questa opzione cancella dei file")).toBeNull();
  expect(component.getSchema().mirror).toBe(false);
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
  const { component } = renderWizard();
  await toOpzioni();
  await fireEvent.click(mirrorSwitch());
  await click("Avanti");

  expect(invokeMock).toHaveBeenCalledWith("preview_sync", {
    source: String.raw`D:\Foto`,
    destination: String.raw`E:\Backup`,
    mode: "Mirror",
  });
  expect((await screen.findByText("Da cancellare in B")).nextElementSibling?.textContent).toBe("12");
  expect(component.getSchema().counts?.deleted).toBe(12);
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
  renderWizard();
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
  const { component } = renderWizard();
  await toOpzioni();
  await fireEvent.click(mirrorSwitch());
  await click("Avanti");

  expect((await screen.findAllByText(reason)).length).toBe(1); // the warning; the schema caption is checked below
  expect(component.getSchema().caption).toBe(reason);
  expect(component.getSchema().caption).not.toContain("B è già aggiornato");
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();
});

test("a blocked preview that still counts files does not say 'Da fare' in the schema", async () => {
  const reason = "Motivo deciso da Rust: non si parte.";
  backend({ preview_sync: () => ({ ...preview(2, 1), blocked: reason, caption: reason }) });
  const { component } = renderWizard();
  await toOpzioni();
  await fireEvent.click(mirrorSwitch());
  await click("Avanti");

  await screen.findAllByText(reason);
  expect(component.getSchema().caption).not.toContain("Da fare");
});

test("a refusal from check_step is shown and Dove stays", async () => {
  backend({
    // Rust refuses a B that is A or inside A (the path rules); the test stands in for that answer.
    check_step: (args) => {
      const { step, destination } = args as { step: string; destination: string };
      if (step !== "destination") return;
      throw {
        title: destination.includes("Backup")
          ? "La destinazione non può stare dentro la cartella da copiare."
          : "La destinazione è la stessa cartella da copiare. Scegline un’altra.",
        advice: "",
      };
    },
  });
  renderWizard();
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
  renderWizard();
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
  renderWizard();
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
  renderWizard();
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
  renderWizard();
  await goToPreview();

  expect(calls("check_step").map(([, args]) => (args as { step: string }).step)).toEqual([
    "source",
    "destination",
    "name",
  ]);
});

test("a run that fails says why in plain Italian, with Riprova and the log link", async () => {
  backend({ preview_sync: () => preview(2, 1), run_sync: () => failedRun });
  renderWizard();
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
  renderWizard();
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
  renderWizard();
  await goToPreview();
  await click(await screen.findByRole("button", { name: "Sì, avvia" }));

  await click(await screen.findByRole("button", { name: "Riprova" }));

  expect(await screen.findByText("Da copiare (nuovi)")).toBeTruthy();
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
  renderWizard();
  await goToPreview();

  expect((await screen.findByRole("alert")).textContent).toContain(failure.title);
  expect(screen.getByText(failure.advice)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Sì, avvia" })).toBeNull();

  await click("Riprova");

  expect(await screen.findByRole("button", { name: "Sì, avvia" })).toBeTruthy();
  expect(calls("preview_sync")).toHaveLength(2);
  expect(screen.queryByRole("alert")).toBeNull();
});
