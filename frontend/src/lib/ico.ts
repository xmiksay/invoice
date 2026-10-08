/**
 * Czech IČO: 8 digits, the last one a mod-11 check digit over weights 8..2.
 * Mirrors the backend rule; the server stays the source of truth.
 */
export function isValidIco(ico: string): boolean {
  if (!/^\d{8}$/.test(ico)) return false;
  let sum = 0;
  for (let i = 0; i < 7; i++) sum += Number(ico[i]) * (8 - i);
  // Remainder 0 → check 1, remainder 1 → check 0, otherwise 11 - remainder.
  const check = (11 - (sum % 11)) % 10;
  return check === Number(ico[7]);
}
