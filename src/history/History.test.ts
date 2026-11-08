import { fireEvent, render, screen, within } from "@testing-library/svelte";
import { invoke } from "@tauri-apps/api/core";
import { expect, test, vi } from "vitest";
import History from "./History.svelte";
import { run } from "../testing/fixtures";

const invokeMock = vi.mocked(invoke);

test("lead and one row per run, in the order given (newest first)", () => {
  render(History, {
    runs: [
      run({ syncName: "Recente", startedAt: "2026-11-03T08:05:00+01:00", when: "03/11/2026 08:05" }),
      run({ syncName: "Vecchia" }),
    ],
    status: "ready",
  });

  expect(
    screen.getByText("Tutte le copie fatte, dalla più recente. Ogni riga ha il suo registro completo."),
  ).toBeTruthy();
  const rows = screen.getAllByRole("listitem");
  expect(rows.map((r) => within(r).getByRole("strong").textContent)).toEqual(["Recente", "Vecchia"]);
  expect(rows[0].textContent).toContain("03/11/2026 08:05");
  expect(rows[1].textContent).toContain("02/11/2026 21:30");
});

test("a row shows mode and outcome chips, counts and duration", () => {
  render(History, {
    runs: [run({ new: 12000, updated: 3, deleted: 5, mode: "Mirror", outcome: "Error" })],
    status: "ready",
  });

  const row = screen.getByRole("listitem");
  expect(within(row).getByText("Specchio")).toBeTruthy();
  expect(within(row).getByText("Errore")).toBeTruthy();
  expect(within(row).getByText("Nuovi").nextElementSibling?.textContent).toBe("12.000");
  expect(within(row).getByText("Aggiornati").nextElementSibling?.textContent).toBe("3");
  expect(within(row).getByText("Cancellati").nextElementSibling?.textContent).toBe("5");
  expect(within(row).getByText("Durata").nextElementSibling?.textContent).toBe("1 min");
});

test("a stopped or failed run shows the files really copied, not the previewed counts", () => {
  render(History, {
    runs: [run({ outcome: "Cancelled", new: 12, updated: 1, partial: 40, partialText: "40 file copiati (parziale)" })],
    status: "ready",
  });

  const row = screen.getByRole("listitem");
  expect(within(row).getByText("40 file copiati (parziale)")).toBeTruthy();
  expect(within(row).queryByText("Nuovi")).toBeNull();
  expect(within(row).queryByText("Aggiornati")).toBeNull();
  expect(within(row).getByText("Durata")).toBeTruthy();
});

test("outcome chips: OK, Errore, Annullato; Copia for copy mode", () => {
  render(History, { runs: [run({ outcome: "Ok" }), run({ outcome: "Cancelled" })], status: "ready" });

  const [first, second] = screen.getAllByRole("listitem");
  expect(within(first).getByText("OK")).toBeTruthy();
  expect(within(first).getByText("Copia")).toBeTruthy();
  expect(within(second).getByText("Annullato")).toBeTruthy();
});

test("while the history is being read there are no rows, only a short message", () => {
  render(History, { runs: [], status: "loading" });

  expect(screen.getByText("Sto leggendo lo storico…")).toBeTruthy();
  expect(screen.queryByRole("listitem")).toBeNull();
});

test("when the history cannot be read, Rust's title and advice say so", () => {
  render(History, {
    runs: [],
    status: "ready",
    error: { title: "Non riesco a leggere lo storico.", advice: "Riprova tra poco." },
  });

  const alert = screen.getByRole("alert").textContent;
  expect(alert).toContain("Non riesco a leggere lo storico.");
  expect(alert).toContain("Riprova tra poco.");
});

test("Registro opens the log modal of that run; Chiudi closes it", async () => {
  invokeMock.mockResolvedValueOnce({ text: "riga di log", errorLines: [] });
  render(History, {
    runs: [run({ syncName: "Altra", logPath: String.raw`C:\L\logs\b.log` })],
    status: "ready",
  });

  await fireEvent.click(screen.getByRole("button", { name: "Registro" }));

  expect(invokeMock).toHaveBeenCalledWith("read_run_log", { path: String.raw`C:\L\logs\b.log` });
  expect(screen.getByRole("dialog", { name: "Registro – Altra (02/11/2026 21:30)" })).toBeTruthy();
  expect(await screen.findByText("riga di log")).toBeTruthy();

  await fireEvent.click(screen.getByRole("button", { name: "Chiudi" }));

  expect(screen.queryByRole("dialog")).toBeNull();
});

test("Esc closes the log modal", async () => {
  invokeMock.mockResolvedValueOnce({ text: "x", errorLines: [] });
  render(History, { runs: [run()], status: "ready" });
  await fireEvent.click(screen.getByRole("button", { name: "Registro" }));

  await fireEvent.keyDown(window, { key: "Escape" });

  expect(screen.queryByRole("dialog")).toBeNull();
});

test("closing the log modal gives the focus back to the Registro button", async () => {
  invokeMock.mockResolvedValueOnce({ text: "x", errorLines: [] });
  render(History, { runs: [run()], status: "ready" });
  const opener = screen.getByRole("button", { name: "Registro" });
  opener.focus();
  await fireEvent.click(opener);
  expect(document.activeElement).toBe(screen.getByRole("button", { name: "Chiudi" }));

  await fireEvent.keyDown(window, { key: "Escape" });

  expect(document.activeElement).toBe(opener);
});

test("Tab stays inside the log modal", async () => {
  invokeMock.mockResolvedValueOnce({ text: "x", errorLines: [] });
  render(History, { runs: [run()], status: "ready" });
  await fireEvent.click(screen.getByRole("button", { name: "Registro" }));
  const close = screen.getByRole("button", { name: "Chiudi" });
  const logText = await screen.findByText("x");

  logText.focus();
  await fireEvent.keyDown(logText, { key: "Tab" });
  expect(document.activeElement).toBe(close);

  await fireEvent.keyDown(close, { key: "Tab", shiftKey: true });
  expect(document.activeElement).toBe(logText);
});

const names = () => screen.queryAllByRole("listitem").map((r) => within(r).getByRole("strong").textContent);
const pick = (label: string, value: string) => fireEvent.change(screen.getByLabelText(label), { target: { value } });

const mixed = () => [
  run({ syncName: "Foto", syncId: 1, outcome: "Ok", mode: "Copy" }),
  run({ syncName: "Documenti", syncId: 2, outcome: "Error", mode: "Mirror" }),
  run({ syncName: "Documenti", syncId: 2, outcome: "Cancelled", mode: "Copy" }),
  run({ syncName: "Foto", syncId: 1, outcome: "Error", mode: "Mirror" }),
];

test("the filters sit above the list, all on 'tutte', with no counter or reset link", () => {
  render(History, { runs: mixed(), status: "ready" });

  expect((screen.getByLabelText("Sincronizzazione") as HTMLSelectElement).selectedOptions[0].textContent).toBe("Tutte");
  expect((screen.getByLabelText("Esito") as HTMLSelectElement).selectedOptions[0].textContent).toBe("Tutti");
  expect((screen.getByLabelText("Modalità") as HTMLSelectElement).selectedOptions[0].textContent).toBe("Tutte");
  expect((screen.getByLabelText("Periodo") as HTMLSelectElement).selectedOptions[0].textContent).toBe("Sempre");
  expect(names()).toHaveLength(4);
  expect(screen.queryByText(/copie su|copia su/)).toBeNull();
  expect(screen.queryByRole("button", { name: "Togli i filtri" })).toBeNull();
});

test("Sincronizzazione lists each sync once and shows only its runs", async () => {
  render(History, { runs: mixed(), status: "ready" });
  const options = within(screen.getByLabelText("Sincronizzazione")).getAllByRole("option");
  expect(options.map((o) => o.textContent)).toEqual(["Tutte", "Documenti", "Foto"]);

  await pick("Sincronizzazione", options[2].getAttribute("value")!);

  expect(names()).toEqual(["Foto", "Foto"]);
});

test("a renamed sync is still one entry: runs are matched by sync id", async () => {
  render(History, {
    runs: [
      run({ syncName: "Nuovo nome", syncId: 1 }),
      run({ syncName: "Vecchio nome", syncId: 1 }),
      run({ syncName: "Altra", syncId: 2 }),
    ],
    status: "ready",
  });
  const options = within(screen.getByLabelText("Sincronizzazione")).getAllByRole("option");
  expect(options).toHaveLength(3);

  await pick("Sincronizzazione", options.find((o) => o.textContent === "Nuovo nome")!.getAttribute("value")!);

  expect(names()).toEqual(["Nuovo nome", "Vecchio nome"]);
});

test("runs without a sync id are matched by name", async () => {
  render(History, { runs: [run({ syncName: "A" }), run({ syncName: "B" }), run({ syncName: "A" })], status: "ready" });
  const options = within(screen.getByLabelText("Sincronizzazione")).getAllByRole("option");
  expect(options.map((o) => o.textContent)).toEqual(["Tutte", "A", "B"]);

  await pick("Sincronizzazione", options[1].getAttribute("value")!);

  expect(names()).toEqual(["A", "A"]);
});

test("Esito shows only OK, Errore or Annullato runs", async () => {
  render(History, { runs: mixed(), status: "ready" });

  await pick("Esito", "Error");
  expect(screen.getAllByRole("listitem").map((r) => within(r).getByText("Errore").textContent)).toEqual([
    "Errore",
    "Errore",
  ]);

  await pick("Esito", "Ok");
  expect(names()).toEqual(["Foto"]);

  await pick("Esito", "Cancelled");
  expect(names()).toEqual(["Documenti"]);
});

test("Modalità shows only Copia or Specchio runs", async () => {
  render(History, { runs: mixed(), status: "ready" });

  await pick("Modalità", "Mirror");
  expect(names()).toEqual(["Documenti", "Foto"]);

  await pick("Modalità", "Copy");
  expect(names()).toEqual(["Foto", "Documenti"]);
});

test("Periodo counts back from now, by the start time of each run", async () => {
  vi.useFakeTimers({ toFake: ["Date"] });
  try {
    vi.setSystemTime(new Date("2026-11-05T12:00:00+01:00"));
    render(History, {
      runs: [
        run({ syncName: "Ieri", startedAt: "2026-11-04T09:00:00+01:00" }),
        run({ syncName: "Otto giorni fa", startedAt: "2026-10-28T09:00:00+01:00" }),
        run({ syncName: "Quaranta giorni fa", startedAt: "2026-09-26T09:00:00+01:00" }),
      ],
      status: "ready",
    });

    await pick("Periodo", "7");
    expect(names()).toEqual(["Ieri"]);

    await pick("Periodo", "30");
    expect(names()).toEqual(["Ieri", "Otto giorni fa"]);

    await pick("Periodo", "");
    expect(names()).toHaveLength(3);
  } finally {
    vi.useRealTimers();
  }
});

test("filters combine: a run must match all of them", async () => {
  render(History, { runs: mixed(), status: "ready" });

  await pick("Esito", "Error");
  await pick("Modalità", "Mirror");
  expect(names()).toEqual(["Documenti", "Foto"]);

  const options = within(screen.getByLabelText("Sincronizzazione")).getAllByRole("option");
  await pick("Sincronizzazione", options[1].getAttribute("value")!);
  expect(names()).toEqual(["Documenti"]);
});

test("with a filter on, a counter and 'Togli i filtri' appear; the link puts everything back", async () => {
  render(History, { runs: mixed(), status: "ready" });

  await pick("Esito", "Error");
  expect(screen.getByText("2 copie su 4")).toBeTruthy();

  await pick("Esito", "Ok");
  expect(screen.getByText("1 copia su 4")).toBeTruthy();

  await pick("Modalità", "Mirror");
  await fireEvent.click(screen.getByRole("button", { name: "Togli i filtri" }));

  expect(names()).toHaveLength(4);
  for (const label of ["Sincronizzazione", "Esito", "Modalità", "Periodo"]) {
    expect((screen.getByLabelText(label) as HTMLSelectElement).value).toBe("");
  }
  expect(screen.queryByText(/copie su|copia su/)).toBeNull();
  expect(screen.queryByRole("button", { name: "Togli i filtri" })).toBeNull();
});

test("when nothing matches, a message replaces the list and the filters can be removed", async () => {
  render(History, { runs: [run({ outcome: "Ok" })], status: "ready" });

  await pick("Esito", "Error");

  expect(screen.getByText("Nessuna copia con questi filtri.")).toBeTruthy();
  expect(screen.getByText("0 copie su 1")).toBeTruthy();
  expect(screen.queryByRole("listitem")).toBeNull();
  await fireEvent.click(screen.getByRole("button", { name: "Togli i filtri" }));
  expect(names()).toHaveLength(1);
  expect(screen.queryByText("Nessuna copia con questi filtri.")).toBeNull();
});

test("the filters stay on while the Registro modal opens and closes", async () => {
  invokeMock.mockResolvedValueOnce({ text: "x", errorLines: [] });
  render(History, { runs: mixed(), status: "ready" });
  await pick("Esito", "Error");

  await fireEvent.click(screen.getAllByRole("button", { name: "Registro" })[0]);
  await fireEvent.keyDown(window, { key: "Escape" });

  expect((screen.getByLabelText("Esito") as HTMLSelectElement).value).toBe("Error");
  expect(names()).toHaveLength(2);
});

test("with no runs there are no filters", () => {
  render(History, { runs: [], status: "ready" });

  expect(screen.queryByLabelText("Esito")).toBeNull();
  expect(screen.queryByText("Nessuna copia con questi filtri.")).toBeNull();
});

test("a deleted log file says it is no longer available", async () => {
  invokeMock.mockRejectedValueOnce({
    title: "Non riesco ad aprire il registro di questa copia.",
    advice: "Controlla che il file ci sia ancora e di avere il permesso di leggerlo.",
  });
  render(History, { runs: [run()], status: "ready" });

  await fireEvent.click(screen.getByRole("button", { name: "Registro" }));

  expect(await screen.findByText(/^Non riesco ad aprire il registro di questa copia\./)).toBeTruthy();
  expect(screen.getByText(/Controlla che il file ci sia ancora/)).toBeTruthy();
});

test("with no period chosen, a run whose start time does not parse is still listed", () => {
  render(History, {
    runs: [run({ syncName: "Strana", startedAt: "non è una data", when: "non è una data" }), run()],
    status: "ready",
  });

  expect(names()).toEqual(["Strana", "Copia di Foto"]);
  expect(screen.getAllByRole("listitem")[0].textContent).toContain("non è una data");
});

test("a sync that ran before and after ids existed is one entry showing all its runs", async () => {
  render(History, {
    runs: [run({ syncName: "Foto", syncId: 1 }), run({ syncName: "Foto" }), run({ syncName: "Altra" })],
    status: "ready",
  });
  const options = within(screen.getByLabelText("Sincronizzazione")).getAllByRole("option");
  expect(options.map((o) => o.textContent)).toEqual(["Tutte", "Altra", "Foto"]);

  await pick("Sincronizzazione", options[2].getAttribute("value")!);

  expect(names()).toEqual(["Foto", "Foto"]);
});
