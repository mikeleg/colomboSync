import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, expect, test, vi } from "vitest";
import { cancelRun, onRunProgress } from "../api";
import type { RunProgress } from "../types";
import StepRunning from "./StepRunning.svelte";

vi.mock("../api", () => ({
  cancelRun: vi.fn(() => Promise.resolve(true)),
  onRunProgress: vi.fn(() => Promise.resolve(() => {})),
}));

const progressAt = (done: number, over: Partial<RunProgress> = {}): RunProgress => ({
  syncName: "Copia di Foto",
  done,
  total: 151,
  elapsedMs: 10_000,
  currentFile: "IMG_0042.jpg",
  ...over,
});

const emit = async (progress: RunProgress) => {
  const handler = vi.mocked(onRunProgress).mock.calls.at(-1)![0];
  handler(progress);
  await Promise.resolve();
};

beforeEach(() => vi.clearAllMocks());

test("before the first event: 0%, 0 di total file, ETA still being computed", () => {
  render(StepRunning, { total: 151 });

  expect(screen.getByRole("heading").textContent).toBe("Sto copiando…");
  expect(screen.getByText("0%")).toBeTruthy();
  expect(screen.getByText("0 di 151 file")).toBeTruthy();
  expect(screen.getByText("Calcolo il tempo che manca…")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Ferma la copia" })).toBeTruthy();
});

test("progress: name, big percentage, N di M file, ETA from the rate so far and the current file", async () => {
  render(StepRunning, { total: 151 });

  await emit(progressAt(40, { elapsedMs: 10_000 })); // 4 files/s, 111 left -> 28 s

  expect(screen.getByRole("heading").textContent).toBe("Sto copiando Copia di Foto…");
  expect(screen.getByText("26%")).toBeTruthy();
  expect(screen.getByText("40 di 151 file")).toBeTruthy();
  expect(screen.getByText("Mancano circa 28 secondi.")).toBeTruthy();
  expect(screen.getByText("Sto copiando: IMG_0042.jpg")).toBeTruthy();
  expect(screen.getByRole("progressbar", { name: "Avanzamento della copia" }).getAttribute("aria-valuenow")).toBe("26");
});

test("ETA says Quasi finito when about a second is left", async () => {
  render(StepRunning, { total: 151 });

  await emit(progressAt(150, { elapsedMs: 30_000 }));

  expect(screen.getByText("Quasi finito.")).toBeTruthy();
});

test("screen readers hear the progress once every 10%", async () => {
  render(StepRunning, { total: 100 });
  const live = () => document.querySelector("[aria-live]")!.textContent;

  await emit(progressAt(4, { total: 100 }));
  expect(live()).toBe("");
  await emit(progressAt(10, { total: 100 }));
  expect(live()).toBe("Copia al 10%");
  await emit(progressAt(19, { total: 100 }));
  expect(live()).toBe("Copia al 10%");
  await emit(progressAt(23, { total: 100 }));
  expect(live()).toBe("Copia al 20%");
});

test("Ferma la copia asks first; No, continua goes back without stopping", async () => {
  render(StepRunning, { total: 151 });

  await fireEvent.click(screen.getByRole("button", { name: "Ferma la copia" }));

  expect(screen.getByRole("alertdialog").textContent).toContain("Vuoi fermare la copia? I file già copiati restano in B.");
  expect(cancelRun).not.toHaveBeenCalled();

  await fireEvent.click(screen.getByRole("button", { name: "No, continua" }));

  expect(screen.queryByRole("alertdialog")).toBeNull();
  expect(screen.getByRole("button", { name: "Ferma la copia" })).toBeTruthy();
  expect(cancelRun).not.toHaveBeenCalled();
});

test("Sì, ferma stops the copy", async () => {
  render(StepRunning, { total: 151 });
  await fireEvent.click(screen.getByRole("button", { name: "Ferma la copia" }));

  await fireEvent.click(screen.getByRole("button", { name: "Sì, ferma" }));

  expect(cancelRun).toHaveBeenCalledWith(false);
});

test("closing the window during the run asks the same question and exits after Sì, ferma", async () => {
  const { rerender } = render(StepRunning, { total: 151, closeRequests: 0 });

  await rerender({ total: 151, closeRequests: 1 });

  expect(await screen.findByRole("alertdialog")).toBeTruthy();
  expect(screen.getByText("Vuoi fermare la copia? I file già copiati restano in B.")).toBeTruthy();
  await fireEvent.click(screen.getByRole("button", { name: "Sì, ferma" }));
  expect(cancelRun).toHaveBeenCalledWith(true);
});

test("an X click before the view was ready still shows the question", async () => {
  render(StepRunning, { total: 151, closeRequests: 1 });

  expect(await screen.findByRole("alertdialog")).toBeTruthy();
});

test("closing the window then No, continua keeps the copy running", async () => {
  render(StepRunning, { total: 151, closeRequests: 1 });
  await fireEvent.click(await screen.findByRole("button", { name: "No, continua" }));

  await fireEvent.click(screen.getByRole("button", { name: "Ferma la copia" }));
  await fireEvent.click(screen.getByRole("button", { name: "Sì, ferma" }));

  expect(cancelRun).toHaveBeenCalledWith(false);
});

test("if there is nothing left to stop the buttons come back", async () => {
  vi.mocked(cancelRun).mockResolvedValueOnce(false);
  render(StepRunning, { total: 151 });
  await fireEvent.click(screen.getByRole("button", { name: "Ferma la copia" }));

  await fireEvent.click(screen.getByRole("button", { name: "Sì, ferma" }));

  expect(await screen.findByRole("button", { name: "Ferma la copia" })).toBeTruthy();
});

test("if stopping fails the buttons come back", async () => {
  vi.mocked(cancelRun).mockRejectedValueOnce("boom");
  render(StepRunning, { total: 151 });
  await fireEvent.click(screen.getByRole("button", { name: "Ferma la copia" }));

  await fireEvent.click(screen.getByRole("button", { name: "Sì, ferma" }));

  expect(await screen.findByRole("button", { name: "Ferma la copia" })).toBeTruthy();
});
