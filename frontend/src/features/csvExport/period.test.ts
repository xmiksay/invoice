import { describe, expect, it } from "vitest";
import { ApiError } from "@/api/client";
import { accountantFallbackName, exportErrorKey, periodErrors, previousMonth } from "./period";

describe("previousMonth", () => {
  it("is the whole previous calendar month", () => {
    expect(previousMonth(new Date(2026, 9, 9))).toEqual({ from: "2026-09-01", to: "2026-09-30" });
    expect(previousMonth(new Date(2026, 4, 31, 23, 59))).toEqual({ from: "2026-04-01", to: "2026-04-30" });
  });

  it("January rolls over to December of the previous year", () => {
    expect(previousMonth(new Date(2026, 0, 1))).toEqual({ from: "2025-12-01", to: "2025-12-31" });
  });

  it("knows leap years", () => {
    expect(previousMonth(new Date(2028, 2, 15))).toEqual({ from: "2028-02-01", to: "2028-02-29" });
  });
});

describe("periodErrors", () => {
  it("requires both bounds and from ≤ to", () => {
    expect(periodErrors("2026-09-01", "2026-09-30")).toEqual({});
    expect(periodErrors("2026-09-01", "2026-09-01")).toEqual({});
    expect(periodErrors("", "")).toEqual({ from: "required", to: "required" });
    expect(periodErrors("2026-10-01", "2026-09-30")).toEqual({ to: "invalid" });
  });
});

describe("exportErrorKey", () => {
  it("explains 422 too_many, unexportable and an empty period, nothing else", () => {
    expect(exportErrorKey(new ApiError(422, "validation", { filter: "too_many" }))).toBe("csvExport.tooMany");
    expect(exportErrorKey(new ApiError(422, "validation", { documents: "unexportable" }, "FV-1"))).toBe("csvExport.unexportable");
    expect(exportErrorKey(new ApiError(422, "validation", { from: "empty" }))).toBe("csvExport.empty");
    expect(exportErrorKey(new ApiError(422, "validation", { from: "invalid" }))).toBeNull();
    expect(exportErrorKey(new ApiError(500, "internal"))).toBeNull();
    expect(exportErrorKey(new Error("x"))).toBeNull();
  });
});

describe("accountantFallbackName", () => {
  it("names the file per format", () => {
    const period = { from: "2026-09-01", to: "2026-09-30", direction: "both" as const };
    expect(accountantFallbackName({ ...period, format: "csv" })).toBe("ucetni-2026-09-01-2026-09-30.csv");
    expect(accountantFallbackName({ ...period, format: "pohoda" })).toBe("pohoda-2026-09-01-2026-09-30.xml");
    expect(accountantFallbackName({ ...period, format: "money" })).toBe("money-2026-09-01-2026-09-30.xml");
  });
});
