import { invoke } from "@tauri-apps/api/core";
import { render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { tick } from "svelte";
import { emptySchema, type SchemaModel } from "../shared/schema";
import SchemaView from "./SchemaView.svelte";

const invokeMock = vi.mocked(invoke);

// What Rust says about each path (see shared/copy_text.rs).
const places: Record<string, { isNetwork: boolean; label: string }> = {
  [String.raw`\\NAS\Backup`]: { isNetwork: true, label: "Rete · NAS" },
  [String.raw`D:\Foto`]: { isNetwork: false, label: "Disco D:" },
  "/Volumes/Backup": { isNetwork: false, label: "Cartella" },
};
beforeEach(() => {
  invokeMock.mockImplementation(async (_command, args) => places[(args as { path: string }).path]);
});

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

test("each path is labelled with what Rust says it is, and a share gets the network icon", async () => {
  const model = { ...picked, source: String.raw`D:\Foto`, destination: String.raw`\\NAS\Backup` };
  const { container } = render(SchemaView, { model });

  expect(await screen.findByText("Rete · NAS")).toBeTruthy();
  expect(screen.getByText("Disco D:")).toBeTruthy();
  expect(invokeMock).toHaveBeenCalledWith("describe_place", { path: String.raw`D:\Foto` });
  expect(invokeMock).toHaveBeenCalledWith("describe_place", { path: String.raw`\\NAS\Backup` });
  const icons = [...container.querySelectorAll(".nb svg")].map((icon) => icon.innerHTML.includes("M7 7.5h.01"));
  expect(icons).toEqual([false, true]);
});

test("a path that is neither a drive nor a share is a Cartella, and each path is asked about once", async () => {
  const { rerender } = render(SchemaView, { model: { ...picked, source: "/Volumes/Backup", destination: "" } });
  expect(await screen.findByText("Cartella")).toBeTruthy();

  await rerender({ model: { ...picked, source: "/Volumes/Backup", destination: String.raw`D:\Foto` } });

  expect(await screen.findByText("Disco D:")).toBeTruthy();
  expect(invokeMock).toHaveBeenCalledTimes(2);
});

describe("places", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  test("a model that carries its places shows them and asks Rust nothing", async () => {
    render(SchemaView, {
      model: {
        ...picked,
        sourcePlace: { isNetwork: false, label: "Disco D:" },
        destinationPlace: { isNetwork: true, label: "Rete · NAS" },
      },
    });
    await vi.advanceTimersByTimeAsync(500);

    expect(screen.getByText("Disco D:")).toBeTruthy();
    expect(screen.getByText("Rete · NAS")).toBeTruthy();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  test("a path typed key by key is asked about once, when it stops changing", async () => {
    const { rerender } = render(SchemaView, { model: { ...picked, source: "", destination: "" } });
    for (const typed of ["D", "D:", String.raw`D:\F`, String.raw`D:\Fo`, String.raw`D:\Foto`]) {
      await rerender({ model: { ...picked, source: typed, destination: "" } });
      await vi.advanceTimersByTimeAsync(50);
    }
    expect(invokeMock).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(200);

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("describe_place", { path: String.raw`D:\Foto` });
    expect(screen.getByText("Disco D:")).toBeTruthy();
  });

  test("storing Rust's answer raises no Svelte warning", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    render(SchemaView, { model: { ...picked, source: String.raw`D:\Foto`, destination: "" } });
    await vi.advanceTimersByTimeAsync(200);

    expect(screen.getByText("Disco D:")).toBeTruthy();
    expect(warn).not.toHaveBeenCalled();
    warn.mockRestore();
  });

  test("a path whose answer is still on its way is not asked about again", async () => {
    invokeMock.mockImplementation(() => new Promise(() => {}));
    const { rerender } = render(SchemaView, { model: { ...picked, destination: "" } });
    await vi.advanceTimersByTimeAsync(200);

    await rerender({ model: { ...picked, destination: "", caption: "Un'altra frase." } });
    await vi.advanceTimersByTimeAsync(200);

    expect(invokeMock).toHaveBeenCalledTimes(1);
  });
});

test("before the folders are chosen it asks for them and shows no numbers", () => {
  render(SchemaView, { model: emptySchema });

  expect(screen.getAllByText("Scegli la cartella")).toHaveLength(2);
  expect(screen.getAllByText("—")).toHaveLength(2);
  expect(screen.queryByText("Nessun file viene cancellato in B")).toBeNull();
});

test("mirror: Specchio chip in its own colour (not Errore's), an Eliminati da B counter and no 'nothing is deleted' reassurance", () => {
  render(SchemaView, { model: { ...picked, mirror: true, counts: { new: 148, updated: 3, deleted: 12 } } });

  expect(screen.getByText("Specchio").className).toContain("mirror");
  expect(screen.getByText("Specchio").className).not.toContain("err");
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
