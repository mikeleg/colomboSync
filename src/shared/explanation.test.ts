import { expect, test } from "vitest";
import { toExplanation } from "./explanation";

const fallback = {
  title: "Qualcosa è andato storto dentro l’app.",
  advice: "Riprova. Se succede ancora, chiudi e riapri ColomboSync.",
};

test("what Rust sent passes through unchanged", () => {
  const sent = { title: "Non riesco a leggere lo storico.", advice: "" };

  expect(toExplanation(sent)).toBe(sent);
  const withAdvice = { title: "Il disco non c’è.", advice: "Il disco è collegato?" };
  expect(toExplanation(withAdvice)).toBe(withAdvice);
});

test("anything else Tauri rejects with becomes the one fixed sentence", () => {
  for (const rejection of [
    "invalid args `consent` for command `run_sync`: missing required key consent",
    new Error("boom"),
    null,
    undefined,
    42,
    {},
    { title: "senza advice" },
    { title: 7, advice: "x" },
  ]) {
    expect(toExplanation(rejection)).toEqual(fallback);
  }
});
