/** `GET /api/health` */
export interface HealthResponse {
  status: string;
  version: string;
}

/** Error body returned by every `/api/*` endpoint. */
export interface ApiErrorBody {
  code: string;
}
