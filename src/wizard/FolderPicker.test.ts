import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, expect, test, vi } from "vitest";
import FolderPicker from "./FolderPicker.svelte";
import type { FolderListing } from "./types";

const invokeMock = vi.mocked(invoke);

const folder = (name: string, path: string) => ({ name, path });
const listing = (parent: string, folders: ReturnType<typeof folder>[] = []): FolderListing => ({
  parent,
  folders,
  error: null,
});
const C = "C:\\";
const D = "D:\\";
const E = "E:\\";
const FOTO = String.raw`D:\Foto`;
const MUSICA = String.raw`D:\Musica`;
const BACKUP = String.raw`E:\Backup`;
const NAS = String.raw`\\NAS\Condivisa`;
const CANNOT_OPEN = "Non riesco ad aprire questa cartella. Il disco o la rete sono collegati?";

const listings: Record<string, FolderListing> = {
  "": listing("", [folder("Disco C:", C), folder("Disco D:", D)]),
  [D]: listing("", [folder("Foto", FOTO), folder("Musica", MUSICA)]),
  [FOTO]: listing(D),
  [BACKUP]: listing(E, [folder("Vecchi", String.raw`E:\Backup\Vecchi`)]),
  [NAS]: { parent: String.raw`\\NAS`, folders: [], error: CANNOT_OPEN },
};

const props = {
  id: "path-a",
  label: "Clicca sulla cartella da copiare (A)",
  placeholder: String.raw`Per esempio: D:\Foto`,
  value: "",
  error: null,
  onenter: () => {},
};

beforeEach(() => {
  invokeMock.mockImplementation(async (_cmd, args) => listings[(args as { path: string }).path] ?? listing(""));
});

test("at the root it asks for a disk and lists the drives", async () => {
  render(FolderPicker, props);

  expect(screen.getByText("Clicca sulla cartella da copiare (A)")).toBeTruthy();
  expect(screen.getByText("Dischi del computer: scegline uno")).toBeTruthy();
  expect(await screen.findByRole("button", { name: "Disco D:" })).toBeTruthy();
  expect(invokeMock).toHaveBeenCalledWith("browse_folders", { path: "" });
  expect(screen.queryByRole("button", { name: /Livello superiore/ })).toBeNull();
});

test("a click chooses the folder and enters it", async () => {
  render(FolderPicker, props);

  await fireEvent.click(await screen.findByRole("button", { name: "Disco D:" }));
  expect(await screen.findByRole("button", { name: "Musica" })).toBeTruthy();
  expect(screen.getByText(D, { selector: "b" })).toBeTruthy();

  await fireEvent.click(screen.getByRole("button", { name: "Foto" }));
  expect(await screen.findByText("Qui dentro non ci sono altre cartelle.")).toBeTruthy();
  expect(screen.getByText(FOTO, { selector: "b" })).toBeTruthy();
  expect((screen.getByLabelText("Percorso") as HTMLInputElement).value).toBe(FOTO);
});

test("Livello superiore goes up, and from a drive back to the list of drives", async () => {
  render(FolderPicker, { ...props, value: FOTO });
  await screen.findByText("Qui dentro non ci sono altre cartelle.");

  await fireEvent.click(screen.getByRole("button", { name: /Livello superiore/ }));
  expect(await screen.findByRole("button", { name: "Musica" })).toBeTruthy();

  await fireEvent.click(screen.getByRole("button", { name: /Livello superiore/ }));
  expect(await screen.findByRole("button", { name: "Disco C:" })).toBeTruthy();
  expect(screen.getByText("Dischi del computer: scegline uno")).toBeTruthy();
});

test("a path typed by hand drives the rows and the parent too", async () => {
  render(FolderPicker, { ...props, value: FOTO });
  await screen.findByText("Qui dentro non ci sono altre cartelle.");

  await fireEvent.input(screen.getByLabelText("Percorso"), { target: { value: BACKUP } });

  expect(await screen.findByRole("button", { name: "Vecchi" })).toBeTruthy();
  expect(screen.getByText(BACKUP, { selector: "b" })).toBeTruthy();
  expect(screen.queryByText("Qui dentro non ci sono altre cartelle.")).toBeNull();

  await fireEvent.click(screen.getByRole("button", { name: /Livello superiore/ }));
  expect((screen.getByLabelText("Percorso") as HTMLInputElement).value).toBe(E);
});

test("a folder that cannot be opened says so, instead of looking empty and chosen", async () => {
  render(FolderPicker, { ...props, value: NAS });

  expect((await screen.findByRole("alert")).textContent).toContain(CANNOT_OPEN);
  expect(screen.queryByText("Qui dentro non ci sono altre cartelle.")).toBeNull();
  expect(screen.queryByText(/Cartella scelta/)).toBeNull();
  expect(document.querySelector(".pickhead.chosen")).toBeNull();
  expect(screen.getByRole("button", { name: /Livello superiore/ })).toBeTruthy();
});

test("when listing the folders is refused, it shows Rust's explanation as it is", async () => {
  invokeMock.mockRejectedValue({ title: CANNOT_OPEN, advice: "Controlla il collegamento e riprova." });
  render(FolderPicker, { ...props, value: NAS });

  expect((await screen.findByRole("alert")).textContent).toBe(`${CANNOT_OPEN} Controlla il collegamento e riprova.`);
});

test("typing a path by hand is tucked away; Enter there behaves as Avanti", async () => {
  const onenter = vi.fn();
  render(FolderPicker, { ...props, onenter });

  const summary = screen.getByText("Preferisci scrivere il percorso a mano?");
  expect(summary.closest("details")?.open).toBe(false);
  expect(screen.getByText(`Per esempio ${FOTO} oppure ${NAS}. Premi Invio per andare avanti.`)).toBeTruthy();

  const input = screen.getByLabelText("Percorso");
  await fireEvent.input(input, { target: { value: NAS } });
  await fireEvent.keyDown(input, { key: "Enter" });

  expect(onenter).toHaveBeenCalledOnce();
});

const NUOVA = String.raw`D:\Nuova`;
const SORGENTE = String.raw`C:\Sorgente`;
const creating = { ...props, id: "path-b", value: D, canCreate: true, source: SORGENTE };

/** Browsing as usual; `create_folder` answers with `create`. */
function answerCreateWith(create: (args: { parent: string; name: string }) => Promise<string>) {
  invokeMock.mockImplementation(async (cmd, args) =>
    cmd === "create_folder"
      ? create(args as { parent: string; name: string })
      : (listings[(args as { path: string }).path] ?? listing("")),
  );
}

test("Nuova cartella is offered only when the picker asks for it, and only inside a folder", async () => {
  const { rerender } = render(FolderPicker, { ...props, value: D });
  await screen.findByRole("button", { name: "Foto" });
  expect(screen.queryByRole("button", { name: /Nuova cartella/ })).toBeNull();

  await rerender({ ...creating, value: "" });
  await screen.findByRole("button", { name: "Disco D:" });
  expect(screen.queryByRole("button", { name: /Nuova cartella/ })).toBeNull();

  await rerender(creating);
  expect(await screen.findByRole("button", { name: /Nuova cartella/ })).toBeTruthy();
});

test("Nuova cartella asks for a name, creates it inside the folder being browsed and chooses it", async () => {
  answerCreateWith(async ({ parent, name }) => `${parent}${name}`);
  render(FolderPicker, creating);

  await fireEvent.click(await screen.findByRole("button", { name: /Nuova cartella/ }));
  await fireEvent.input(screen.getByLabelText("Nome della nuova cartella"), { target: { value: "Nuova" } });
  await fireEvent.click(screen.getByRole("button", { name: "Crea" }));

  expect(invokeMock).toHaveBeenCalledWith("create_folder", { source: SORGENTE, parent: D, name: "Nuova" });
  expect(await screen.findByText(NUOVA, { selector: "b" })).toBeTruthy();
  expect((screen.getByLabelText("Percorso") as HTMLInputElement).value).toBe(NUOVA);
  expect(screen.queryByLabelText("Nome della nuova cartella")).toBeNull();
  expect(screen.getByRole("button", { name: /Nuova cartella/ })).toBeTruthy();
});

test("Enter in the name creates the folder too", async () => {
  answerCreateWith(async ({ parent, name }) => `${parent}${name}`);
  render(FolderPicker, creating);

  await fireEvent.click(await screen.findByRole("button", { name: /Nuova cartella/ }));
  const name = screen.getByLabelText("Nome della nuova cartella");
  await fireEvent.input(name, { target: { value: "Nuova" } });
  await fireEvent.keyDown(name, { key: "Enter" });

  expect(await screen.findByText(NUOVA, { selector: "b" })).toBeTruthy();
});

test("a refused name shows Rust's message next to the input and keeps the folder and the name", async () => {
  answerCreateWith(() =>
    Promise.reject({ title: "Esiste già una cartella con questo nome.", advice: "Scegli un altro nome." }),
  );
  render(FolderPicker, creating);

  await fireEvent.click(await screen.findByRole("button", { name: /Nuova cartella/ }));
  await fireEvent.input(screen.getByLabelText("Nome della nuova cartella"), { target: { value: "Foto" } });
  await fireEvent.click(screen.getByRole("button", { name: "Crea" }));

  expect((await screen.findByRole("alert")).textContent).toContain(
    "Esiste già una cartella con questo nome. Scegli un altro nome.",
  );
  expect((screen.getByLabelText("Nome della nuova cartella") as HTMLInputElement).value).toBe("Foto");
  expect(screen.getByText(D, { selector: "b" })).toBeTruthy();
  expect((screen.getByLabelText("Percorso") as HTMLInputElement).value).toBe(D);
});

test("Annulla closes the name without creating anything, and so does leaving the folder", async () => {
  render(FolderPicker, creating);

  await fireEvent.click(await screen.findByRole("button", { name: /Nuova cartella/ }));
  await fireEvent.click(screen.getByRole("button", { name: "Annulla" }));
  expect(screen.queryByLabelText("Nome della nuova cartella")).toBeNull();

  await fireEvent.click(screen.getByRole("button", { name: /Nuova cartella/ }));
  await fireEvent.click(await screen.findByRole("button", { name: "Foto" }));
  await vi.waitFor(() => expect(screen.queryByLabelText("Nome della nuova cartella")).toBeNull());

  expect(invokeMock).not.toHaveBeenCalledWith("create_folder", expect.anything());
});

test("typing another path closes the name box, and coming back to the folder does not bring it back", async () => {
  render(FolderPicker, creating);

  await fireEvent.click(await screen.findByRole("button", { name: /Nuova cartella/ }));
  await fireEvent.input(screen.getByLabelText("Nome della nuova cartella"), { target: { value: "Vecchio" } });
  await fireEvent.input(screen.getByLabelText("Percorso"), { target: { value: FOTO } });
  expect(screen.queryByLabelText("Nome della nuova cartella")).toBeNull();

  await fireEvent.click(await screen.findByRole("button", { name: /Livello superiore/ }));
  await screen.findByRole("button", { name: "Foto" });
  expect(screen.queryByLabelText("Nome della nuova cartella")).toBeNull();
});

test("a step error opens the manual section and focuses the input", async () => {
  const { rerender } = render(FolderPicker, props);
  const details = screen.getByText("Preferisci scrivere il percorso a mano?").closest("details")!;
  expect(details.open).toBe(false);

  await rerender({ ...props, error: { title: "Scegli prima la cartella da copiare.", advice: "" } });

  expect(screen.getByRole("alert").textContent).toContain("Scegli prima la cartella da copiare.");
  expect(details.open).toBe(true);
  await vi.waitFor(() => expect(document.activeElement).toBe(screen.getByLabelText("Percorso")));
  expect(screen.getByLabelText("Percorso").getAttribute("aria-invalid")).toBe("true");
});
