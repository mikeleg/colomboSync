import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, expect, test, vi } from "vitest";
import FolderPicker from "./FolderPicker.svelte";
import type { FolderListing } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
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
  error: "",
  onenter: () => {},
};

beforeEach(() => {
  invokeMock.mockReset();
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

test("a step error opens the manual section and focuses the input", async () => {
  const { rerender } = render(FolderPicker, props);
  const details = screen.getByText("Preferisci scrivere il percorso a mano?").closest("details")!;
  expect(details.open).toBe(false);

  await rerender({ ...props, error: "Scegli prima la cartella da copiare." });

  expect(screen.getByRole("alert").textContent).toContain("Scegli prima la cartella da copiare.");
  expect(details.open).toBe(true);
  await vi.waitFor(() => expect(document.activeElement).toBe(screen.getByLabelText("Percorso")));
  expect(screen.getByLabelText("Percorso").getAttribute("aria-invalid")).toBe("true");
});
