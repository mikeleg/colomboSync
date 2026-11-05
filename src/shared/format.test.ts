import { expect, test } from "vitest";
import { formatWhen } from "./format";

test("formatWhen shows the local date and time written in the RFC 3339 string", () => {
  expect(formatWhen("2026-11-02T21:30:00+01:00")).toBe("02/11/2026 21:30");
  expect(formatWhen("2026-11-02T21:30:00.123456+01:00")).toBe("02/11/2026 21:30");
});

test("formatWhen shows an unexpected value as it is", () => {
  expect(formatWhen("ieri sera")).toBe("ieri sera");
});
