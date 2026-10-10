/** `?return=` target after sign-in: only same-app absolute paths ("//host" would be protocol-relative). */
export function safeReturnPath(target: unknown): string {
  return typeof target === "string" && target.startsWith("/") && !target.startsWith("//") ? target : "/";
}
