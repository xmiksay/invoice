import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApiError, type BlobResponse } from "@/api/client";
import { downloadPdf, openPdf, REVOKE_AFTER_MS } from "./pdf";

const pdf = (filename: string | null = null): BlobResponse => ({ blob: new Blob(["%PDF"]), filename });

describe("pdf helpers", () => {
  let clicked: HTMLAnchorElement[];

  beforeEach(() => {
    vi.useFakeTimers({ toFake: ["setTimeout"] });
    clicked = [];
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) {
      clicked.push(this);
    });
    vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: vi.fn(() => "blob:pdf-1"), revokeObjectURL: vi.fn() }));
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("opens the tab before loading, then points it at the object URL and revokes later", async () => {
    const tab = { closed: false, location: { href: "" }, close: vi.fn() };
    const open = vi.fn(() => tab);
    vi.stubGlobal("open", open);
    let openedBeforeLoad = false;

    await openPdf(async () => {
      openedBeforeLoad = open.mock.calls.length === 1;
      return pdf();
    });

    expect(openedBeforeLoad).toBe(true);
    expect(open).toHaveBeenCalledWith("", "_blank");
    expect(tab.location.href).toBe("blob:pdf-1");
    expect(clicked).toEqual([]);
    expect(URL.revokeObjectURL).not.toHaveBeenCalled();
    vi.advanceTimersByTime(REVOKE_AFTER_MS);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:pdf-1");
  });

  it("navigates the current tab when the popup is blocked", async () => {
    vi.stubGlobal("open", vi.fn(() => null));
    await openPdf(async () => pdf());
    expect(clicked).toHaveLength(1);
    expect(clicked[0]?.href).toBe("blob:pdf-1");
    expect(clicked[0]?.hasAttribute("download")).toBe(false);
  });

  it("does nothing when the user closed the tab while the PDF was loading", async () => {
    const tab = { closed: false, location: { href: "" }, close: vi.fn() };
    vi.stubGlobal("open", vi.fn(() => tab));
    await openPdf(async () => {
      tab.closed = true;
      return pdf();
    });
    expect(tab.location.href).toBe("");
    expect(clicked).toEqual([]);
    expect(URL.createObjectURL).not.toHaveBeenCalled();
  });

  it("closes the blank tab and rethrows when loading fails", async () => {
    const tab = { closed: false, location: { href: "" }, close: vi.fn() };
    vi.stubGlobal("open", vi.fn(() => tab));
    const err = new ApiError(503, "pdf_unavailable");
    await expect(openPdf(async () => Promise.reject(err))).rejects.toBe(err);
    expect(tab.close).toHaveBeenCalledOnce();
  });

  it("downloads under the server's filename, else the fallback", async () => {
    await downloadPdf(async () => pdf("20260001.pdf"), "x.pdf");
    await downloadPdf(async () => pdf(null), "draft-abcd1234.pdf");
    expect(clicked.map((a) => a.download)).toEqual(["20260001.pdf", "draft-abcd1234.pdf"]);
    expect(document.body.querySelector("a")).toBeNull();
  });
});
