/** `GET /api/health` */
export interface HealthResponse {
  status: string;
  version: string;
}

/** Field name (camelCase wire name) → reason code, e.g. `{ ico: "invalid_ico" }`. */
export type FieldErrors = Record<string, string>;

/** Error body returned by every `/api/*` endpoint; `fields` only on 422 `validation`. */
export interface ApiErrorBody {
  code: string;
  fields?: FieldErrors;
}

export type DocLocale = "cs" | "en";

/** `GET /api/ares/{ico}` — a contact draft from the Czech business register. */
export interface AresSubject {
  name: string;
  ico: string;
  dic: string | null;
  street: string;
  city: string;
  zip: string;
  country: string;
}
