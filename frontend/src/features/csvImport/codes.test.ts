import { describe, expect, it } from "vitest";
import { ApiError } from "@/api/client";
import { i18n } from "@/i18n";
import { FILE_ERRORS, pickFile, ROW_ERRORS, rowErrorKey, uploadError, WARNINGS, warningKey } from "./codes";
import { csvFile } from "./testData";

describe("csv import codes", () => {
  it("takes exactly one .csv / .txt / .xlsx file (any case)", () => {
    for (const name of ["a.csv", "B.TXT", "c.xlsx"]) expect(pickFile([csvFile(name)])?.name).toBe(name);
    expect(pickFile([csvFile("a.xls")])).toBeNull();
    expect(pickFile([csvFile("a.csv"), csvFile("b.csv")])).toBeNull();
    expect(pickFile([])).toBeNull();
  });

  it("maps known row / warning codes and falls back for unknown ones; every key exists", () => {
    expect(rowErrorKey("total_mismatch")).toBe("csvImport.rowError.total_mismatch");
    expect(rowErrorKey("new_thing")).toBe("csvImport.rowError.unknown");
    expect(warningKey("category_inactive")).toBe("csvImport.warning.category_inactive");
    expect(warningKey("other")).toBe("csvImport.warning.unknown");
    for (const locale of ["cs", "en"] as const) {
      for (const code of [...ROW_ERRORS, "x"]) expect(i18n.global.te(rowErrorKey(code), locale)).toBe(true);
      for (const code of [...WARNINGS, "x"]) expect(i18n.global.te(warningKey(code), locale)).toBe(true);
      for (const code of [...FILE_ERRORS, "x"]) {
        expect(i18n.global.te(uploadError(new ApiError(422, "validation", { file: code }))!.key, locale)).toBe(true);
      }
    }
  });

  it("narrows not_allowed by the column where the reason is unambiguous", () => {
    expect(rowErrorKey("not_allowed", "tax_date")).toBe("csvImport.notAllowed.taxDate");
    expect(rowErrorKey("not_allowed", "paid_date")).toBe("csvImport.notAllowed.paidDate");
    expect(rowErrorKey("not_allowed", "vat_21")).toBe("csvImport.notAllowed.vat");
    expect(rowErrorKey("not_allowed", "base_12_5")).toBe("csvImport.notAllowed.vat");
    expect(rowErrorKey("not_allowed", "counterparty_name")).toBe("csvImport.notAllowed.counterparty");
    expect(rowErrorKey("not_allowed", "currency")).toBe("csvImport.rowError.not_allowed");
    expect(rowErrorKey("not_allowed")).toBe("csvImport.rowError.not_allowed");
    expect(rowErrorKey("invalid_value", "tax_date")).toBe("csvImport.rowError.invalid_value");
    for (const locale of ["cs", "en"] as const) {
      for (const field of ["tax_date", "paid_date", "vat_21", "counterparty_name"]) {
        expect(i18n.global.te(rowErrorKey("not_allowed", field), locale)).toBe(true);
      }
    }
  });

  it("maps the 413 and the file-level 422 with its column detail", () => {
    expect(uploadError(new ApiError(413, "too_large"))).toEqual({ key: "csvImport.upload.tooLarge", params: {} });
    expect(uploadError(new ApiError(422, "validation", { file: "missing_column" }, "supplier_number"))).toEqual({
      key: "csvImport.fileError.missing_column",
      params: { code: "missing_column", column: "supplier_number" },
    });
    expect(uploadError(new ApiError(422, "validation", { file: "weird" }))?.key).toBe("csvImport.fileError.unknown");
    expect(uploadError(new ApiError(422, "validation", { other: "x" }))).toBeNull();
    expect(uploadError(new ApiError(500, "internal"))).toBeNull();
    expect(uploadError(new Error("x"))).toBeNull();
  });
});
