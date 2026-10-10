import QRCode from "qrcode";

/**
 * The QR code as an SVG data URI, rendered in the browser so the TOTP secret never leaves it
 * (no image service); SVG needs no canvas and scales crisply.
 */
export async function qrDataUri(text: string): Promise<string> {
  const svg = await QRCode.toString(text, { type: "svg", errorCorrectionLevel: "M", margin: 2 });
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}
