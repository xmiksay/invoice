import { request } from "@/api/client";
import type { ApiToken, CreatedToken, CreateTokenBody } from "./types";

export const tokensApi = {
  list: () => request<ApiToken[]>("/api/tokens"),
  create: (body: CreateTokenBody) => request<CreatedToken>("/api/tokens", { method: "POST", body }),
  revoke: (id: string) => request<void>(`/api/tokens/${encodeURIComponent(id)}`, { method: "DELETE" }),
};
