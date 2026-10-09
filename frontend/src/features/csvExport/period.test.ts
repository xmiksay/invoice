import { describe, expect, it } from "vitest";
import { ApiError } from "@/api/client";
import { exportErrorKey, periodErrors, previousMonth } from "./period";

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
  it("explains only 422 filter too_many", () => {
    expect(exportErrorKey(new ApiError(422, "validation", { filter: "too_many" }))).toBe("csvExport.tooMany");
    expect(exportErrorKey(new ApiError(422, "validation", { from: "invalid" }))).toBeNull();
    expect(exportErrorKey(new ApiError(500, "internal"))).toBeNull();
    expect(exportErrorKey(new Error("x"))).toBeNull();
  });
});
