/**
 * Client mirror of the backend `format_number` / `validate_pattern` for the live
 * preview. Rules: tokens `{YYYY}`, `{YY}`, `{N…}` (1–9 N's = zero-padded width),
 * exactly one year token (`{YYYY}` or `{YY}`), exactly one `{N…}`, literals
 * `[A-Za-z0-9/_-]`, at most 40 chars.
 */
export type PatternToken =
  | { kind: "literal"; text: string }
  | { kind: "yyyy" }
  | { kind: "yy" }
  | { kind: "n"; width: number };

export const MAX_PATTERN_LENGTH = 40;
const LITERAL = /^[A-Za-z0-9/_-]$/;
const COUNTER = /^N{1,9}$/;

/** Tokens of a valid pattern, or null when the backend would answer `invalid_pattern`. */
export function parsePattern(pattern: string): PatternToken[] | null {
  if (pattern.length > MAX_PATTERN_LENGTH) return null;
  const tokens: PatternToken[] = [];
  let counters = 0;
  let years = 0;
  let i = 0;
  while (i < pattern.length) {
    const ch = pattern.charAt(i);
    if (ch === "{") {
      const end = pattern.indexOf("}", i);
      if (end < 0) return null;
      const name = pattern.slice(i + 1, end);
      if (name === "YYYY" || name === "YY") {
        years++;
        tokens.push({ kind: name === "YYYY" ? "yyyy" : "yy" });
      }
      else if (COUNTER.test(name)) {
        counters++;
        tokens.push({ kind: "n", width: name.length });
      } else return null;
      i = end + 1;
    } else if (LITERAL.test(ch)) {
      const last = tokens[tokens.length - 1];
      if (last?.kind === "literal") last.text += ch;
      else tokens.push({ kind: "literal", text: ch });
      i++;
    } else {
      return null;
    }
  }
  return counters === 1 && years === 1 ? tokens : null;
}

export function isValidPattern(pattern: string): boolean {
  return parsePattern(pattern) !== null;
}

/** Formatted number, or null for an invalid pattern. A counter wider than its token is not truncated. */
export function formatNumber(pattern: string, year: number, n: number): string | null {
  const tokens = parsePattern(pattern);
  if (!tokens) return null;
  return tokens
    .map((tok) => {
      switch (tok.kind) {
        case "literal":
          return tok.text;
        case "yyyy":
          return String(year).padStart(4, "0");
        case "yy":
          return String(year % 100).padStart(2, "0");
        case "n":
          return String(n).padStart(tok.width, "0");
      }
    })
    .join("");
}
