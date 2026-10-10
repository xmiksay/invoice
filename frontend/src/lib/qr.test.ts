import { describe, expect, it } from "vitest";
import { qrDataUri } from "./qr";

describe("qrDataUri", () => {
  it("renders an SVG data URI", async () => {
    const uri = await qrDataUri("otpauth://totp/Invoice:jana@example.cz?secret=JBSWY3DPEHPK3PXP&issuer=Invoice");
    expect(uri.startsWith("data:image/svg+xml;charset=utf-8,")).toBe(true);
    expect(decodeURIComponent(uri)).toContain("<svg");
  });
});
