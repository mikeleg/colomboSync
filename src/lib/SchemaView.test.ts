import { render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { tick } from "svelte";
import { emptySchema, type SchemaModel } from "./schema";
import SchemaView from "./SchemaView.svelte";

const picked: SchemaModel = {
  source: "D:\Foto",
  destination: "E:\Backup",
  counts: { new: 148, updated: 3, deleted: 0 },
  caption: "Da fare: copiare 148 file nuovi, aggiornarne 3.",
  mirror: false,
  warnCaption: false,
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

test("mirror: red Specchio chip, an Eliminati da B counter and no 'nothing is deleted' reassurance", () => {
  render(SchemaView, { model: { ...picked, mirror: true, counts: { new: 148, updated: 3, deleted: 12 } } });

  expect(screen.getByText("Specchio").className).toContain("mirror");
  expect(screen.getByText("12").nextElementSibling?.textContent).toBe("Eliminati da B");
  expect(screen.queryByText("Nessun file viene cancellato in B")).toBeNull();
  expect(screen.queryByText("Copia", { selector: ".chip" })).toBeNull();
});

test("copy: no Eliminati counter", () => {
  render(SchemaView, { model: picked });

  expect(screen.getByText("Copia", { selector: ".chip" })).toBeTruthy();
  expect(screen.queryByText("Eliminati da B")).toBeNull();
});

test("while a run is going the arrow is animated", () => {
  const { container } = render(SchemaView, { model: { ...picked, running: true } });

  expect(container.querySelector(".mid.run")).toBeTruthy();
});

test("the arrow is still when nothing runs", () => {
  const { container } = render(SchemaView, { model: picked });

  expect(container.querySelector(".mid.run")).toBeNull();
});

describe("motion", () => {
  const frames: ((now: number) => void)[] = [];
  const motion = (noPreference: boolean) =>
    vi.stubGlobal("matchMedia", (query: string) => ({ matches: noPreference && query.includes("no-preference") }));

  beforeEach(() => {
    frames.length = 0;
    vi.stubGlobal("requestAnimationFrame", (cb: (now: number) => void) => frames.push(cb));
    vi.stubGlobal("cancelAnimationFrame", () => {});
  });
  afterEach(() => vi.unstubAllGlobals());

  test("counters count up from 0 and end on the exact value", async () => {
    motion(true);
    const { rerender } = render(SchemaView, { model: { ...picked, counts: null } });
    await rerender({ model: picked });

    frames.shift()!(0); // first frame: the count starts from 0
    await tick();
    expect(screen.queryByText("148")).toBeNull();
    expect(screen.getAllByText("0").length).toBeGreaterThan(0);

    for (let t = 100; frames.length && t < 5000; t += 100) {
      frames.shift()!(t);
      await tick();
    }
    expect(screen.getByText("148").nextElementSibling?.textContent).toBe("Nuovi");
    expect(screen.getByText("3").nextElementSibling?.textContent).toBe("Aggiornati");
  });

  test("with reduced motion the values show at once", async () => {
    motion(false);
    const { rerender } = render(SchemaView, { model: { ...picked, counts: null } });
    await rerender({ model: picked });

    expect(frames).toHaveLength(0);
    expect(screen.getByText("148")).toBeTruthy();
  });
});

test("mirror adds the deleted-particles element, copy does not", () => {
  const mirror = render(SchemaView, { model: { ...picked, mirror: true } });
  expect(mirror.container.querySelector(".gone")?.getAttribute("aria-hidden")).toBe("true");
  mirror.unmount();

  const { container } = render(SchemaView, { model: picked });
  expect(container.querySelector(".gone")).toBeNull();
});

test("the arrow has decorative drifting dots", () => {
  const { container } = render(SchemaView, { model: picked });

  expect(container.querySelectorAll(".dot")).toHaveLength(3);
  expect(container.querySelector(".arr svg")?.getAttribute("aria-hidden")).toBe("true");
});

test("running adds the run class to the arrow and to both tiles", () => {
  const { container } = render(SchemaView, { model: { ...picked, running: true } });

  expect(container.querySelector(".mid.run")).toBeTruthy();
  expect(container.querySelectorAll(".node.run")).toHaveLength(2);
});
