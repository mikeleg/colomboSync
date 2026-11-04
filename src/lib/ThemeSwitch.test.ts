import { fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import ThemeSwitch from "./ThemeSwitch.svelte";

const store = new Map<string, string>();

beforeEach(() => {
  store.clear();
  vi.stubGlobal("localStorage", {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
  });
  document.documentElement.removeAttribute("data-theme");
});
afterEach(() => vi.unstubAllGlobals());

const pressed = (name: string) => screen.getByRole("button", { name, pressed: true });

test("starts on Auto, which leaves the Windows setting in charge", () => {
  render(ThemeSwitch);

  expect(pressed("Auto")).toBeTruthy();
  expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
});

test("Scuro applies the dark tokens at once and is remembered", async () => {
  render(ThemeSwitch);

  await fireEvent.click(screen.getByRole("button", { name: "Scuro" }));

  expect(pressed("Scuro")).toBeTruthy();
  expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  expect(store.get("cs-theme")).toBe("dark");
});

test("after a restart the saved choice is applied", () => {
  store.set("cs-theme", "dark");

  render(ThemeSwitch);

  expect(pressed("Scuro")).toBeTruthy();
  expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
});

test("Chiaro forces light; going back to Auto removes the override", async () => {
  render(ThemeSwitch);

  await fireEvent.click(screen.getByRole("button", { name: "Chiaro" }));
  expect(document.documentElement.getAttribute("data-theme")).toBe("light");

  await fireEvent.click(screen.getByRole("button", { name: "Auto" }));
  expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
});

test("works when localStorage is not available", async () => {
  vi.stubGlobal("localStorage", {
    getItem: () => {
      throw new Error("blocked");
    },
    setItem: () => {
      throw new Error("blocked");
    },
  });
  render(ThemeSwitch);

  await fireEvent.click(screen.getByRole("button", { name: "Scuro" }));

  expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
});
