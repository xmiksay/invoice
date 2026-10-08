import { request } from "./client";
import type { AresSubject } from "./types";

export const aresApi = {
  lookup(ico: string): Promise<AresSubject> {
    return request<AresSubject>(`/api/ares/${encodeURIComponent(ico)}`);
  },
};
