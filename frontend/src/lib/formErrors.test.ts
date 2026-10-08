import { describe, expect, it } from "vitest";
import { ApiError } from "@/api/client";
import {
  collectErrors,
  errorMessageKey,
  fieldErrorsOf,
  nullIfEmpty,
  reasonKey,
  textRule,
} from "./formErrors";

describe("fieldErrorsOf", () => {
  it("maps a 422 validation response to field errors", () => {
    const err = new ApiError(422, "validation", { ico: "duplicate", name: "required" });
    expect(fieldErrorsOf(err)).toEqual({ ico: "duplicate", name: "required" });
  });

  it("returns a copy, not the error's own object", () => {
    const err = new ApiError(422, "validation", { ico: "duplicate" });
    const fields = fieldErrorsOf(err)!;
    fields.ico = "x";
    expect(err.fields.ico).toBe("duplicate");
  });

  it("returns null for non-422 errors and 422 without fields", () => {
    expect(fieldErrorsOf(new ApiError(409, "conflict"))).toBeNull();
    expect(fieldErrorsOf(new ApiError(422, "validation"))).toBeNull();
    expect(fieldErrorsOf(new Error("boom"))).toBeNull();
  });
});

describe("reasonKey", () => {
  it("maps known reason codes and falls back to invalid", () => {
    expect(reasonKey("invalid_ico")).toBe("validation.invalid_ico");
    expect(reasonKey("too_long")).toBe("validation.too_long");
    expect(reasonKey("something_new")).toBe("validation.invalid");
  });
});

describe("errorMessageKey", () => {
  it("maps known codes, unreachable statuses and unknown codes", () => {
    expect(errorMessageKey(new ApiError(409, "conflict")).key).toBe("errors.conflict");
    expect(errorMessageKey(new ApiError(502, "ares_unavailable")).key).toBe("errors.aresUnavailable");
    expect(errorMessageKey(new ApiError(0, "network")).key).toBe("errors.unreachable");
    expect(errorMessageKey(new ApiError(500, "internal"))).toEqual({
      key: "errors.unexpected",
      params: { code: "internal" },
    });
  });
});

describe("helpers", () => {
  it("collectErrors drops passing checks", () => {
    expect(collectErrors({ a: null, b: "required", c: false })).toEqual({ b: "required" });
  });

  it("textRule checks required and length on trimmed input", () => {
    expect(textRule("  ", { required: true })).toBe("required");
    expect(textRule("abc", { max: 2 })).toBe("too_long");
    expect(textRule(" ab ", { required: true, max: 2 })).toBeNull();
  });

  it("nullIfEmpty trims and nulls blanks", () => {
    expect(nullIfEmpty("  ")).toBeNull();
    expect(nullIfEmpty(" x ")).toBe("x");
  });
});
