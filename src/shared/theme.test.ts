import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { initTheme } from "./theme";

const setItem = vi.fn();

beforeEach(() => {
  setItem.mockReset();
  document.documentElement.removeAttribute("data-theme");
});
afterEach(() => vi.unstubAllGlobals());

const storage = (saved: string | null) => vi.stubGlobal("localStorage", { getItem: () => saved, setItem });

test("at startup the saved theme is applied without writing anything back", () => {
  storage("dark");

  initTheme();

  expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  expect(setItem).not.toHaveBeenCalled();
});

test("at startup with nothing saved Auto stays in charge", () => {
  storage(null);

  initTheme();

  expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  expect(setItem).not.toHaveBeenCalled();
});
