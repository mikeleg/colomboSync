import { render } from "@testing-library/svelte";
import { expect, test } from "vitest";
import ErrorText from "./ErrorText.svelte";

const textOf = (explanation: { title: string; advice: string }) => {
  const { container } = render(ErrorText, { explanation });
  return container.textContent;
};

test("title and advice are one sentence apart by a single space", () => {
  expect(textOf({ title: "Non riesco a leggere lo storico.", advice: "Riprova tra poco." })).toBe(
    "Non riesco a leggere lo storico. Riprova tra poco.",
  );
});

test("no advice, no trailing space", () => {
  expect(textOf({ title: "Non riesco a leggere lo storico.", advice: "" })).toBe("Non riesco a leggere lo storico.");
});
