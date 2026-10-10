import { request } from "@/api/client";
import type { AppContext, CreateSpaceBody, DeleteSpaceBody, Space } from "./types";

export const spacesApi = {
  context: () => request<AppContext>("/api/context", { quiet401: true }),
  // Base host
  list: () => request<Space[]>("/api/spaces"),
  create: (body: CreateSpaceBody) => request<Space>("/api/spaces", { method: "POST", body }),
  // Space host
  current: () => request<Space>("/api/space"),
  rename: (name: string) => request<Space>("/api/space", { method: "PUT", body: { name } }),
  remove: (body: DeleteSpaceBody) => request<void>("/api/space", { method: "DELETE", body }),
};
