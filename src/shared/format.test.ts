import { expect, test } from "vitest";
import { formatNumber, formatSize } from "./format";

test("numbers use the Italian thousands separator", () => {
  expect(formatNumber(999)).toBe("999");
  expect(formatNumber(12345)).toBe("12.345");
});

test("sizes: whole megabytes up to a gigabyte, then one decimal with a comma", () => {
  expect(formatSize(0)).toBe("1 MB");
  expect(formatSize(5_000_000)).toBe("5 MB");
  expect(formatSize(1_500_000_000)).toBe("1,5 GB");
  expect(formatSize(1_234_500_000_000)).toBe("1234,5 GB");
});
