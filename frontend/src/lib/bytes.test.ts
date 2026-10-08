import { describe, expect, it } from "vitest";
import { formatBytes } from "./bytes";

describe("formatBytes", () => {
  it("scales by 1024 with one decimal above bytes", () => {
    expect(formatBytes(0, "en")).toBe("0 B");
    expect(formatBytes(1023, "en")).toBe("1,023 B");
    expect(formatBytes(1536, "en")).toBe("1.5 kB");
    expect(formatBytes(2048, "en")).toBe("2 kB");
    expect(formatBytes(5 * 1024 * 1024, "en")).toBe("5 MB");
  });

  it("uses the locale's decimal separator", () => {
    expect(formatBytes(1536, "cs")).toBe("1,5 kB");
  });
});
