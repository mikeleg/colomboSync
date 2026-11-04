import { render, screen } from "@testing-library/svelte";
import { expect, test } from "vitest";
import { emptySchema, type SchemaModel } from "./schema";
import SchemaView from "./SchemaView.svelte";

const picked: SchemaModel = {
  source: "D:\Foto",
  destination: "E:\Backup",
  counts: { new: 148, updated: 3, deleted: 0 },
  caption: "Da fare: copiare 148 file nuovi, aggiornarne 3.",
};

test("shows A, B, the counts and that nothing is deleted in B", () => {
  render(SchemaView, { model: picked });

  expect(screen.getByText("D:\Foto")).toBeTruthy();
  expect(screen.getByText("E:\Backup")).toBeTruthy();
  expect(screen.getByText("148").nextElementSibling?.textContent).toBe("Nuovi");
  expect(screen.getByText("3").nextElementSibling?.textContent).toBe("Aggiornati");
  expect(screen.getByText("Nessun file viene cancellato in B")).toBeTruthy();
  expect(screen.getByText(picked.caption)).toBeTruthy();
});

test("before the folders are chosen it asks for them and shows no numbers", () => {
  render(SchemaView, { model: emptySchema });

  expect(screen.getAllByText("Scegli la cartella")).toHaveLength(2);
  expect(screen.getAllByText("—")).toHaveLength(2);
  expect(screen.queryByText("Nessun file viene cancellato in B")).toBeNull();
});

test("while a run is going the arrow is animated", () => {
  const { container } = render(SchemaView, { model: { ...picked, running: true } });

  expect(container.querySelector(".mid.run")).toBeTruthy();
});

test("the arrow is still when nothing runs", () => {
  const { container } = render(SchemaView, { model: picked });

  expect(container.querySelector(".mid.run")).toBeNull();
});
