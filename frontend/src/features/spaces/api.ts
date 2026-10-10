import { request } from "@/api/client";
import type { AppContext, CreateSpaceBody, DeleteSpaceBody, Space, UpdateSpaceBody } from "./types";

export const spacesApi = {
  context: () => request<AppContext>("/api/context", { quiet401: true }),
  // Base host
  list: () => request<Space[]>("/api/spaces"),
  create: (body: CreateSpaceBody) => request<Space>("/api/spaces", { method: "POST", body }),
  // Space host
  current: () => request<Space>("/api/space"),
  update: (body: UpdateSpaceBody) => request<Space>("/api/space", { method: "PUT", body }),
  remove: (body: DeleteSpaceBody) => request<void>("/api/space", { method: "DELETE", body }),
};
