import type { Direction } from "@/features/documents/types";

/** `direction` of `GET /api/export/accountant`. */
export type AccountantDirection = Direction | "both";

/** Query of `GET /api/export/accountant`: a tax-date period, both bounds inclusive. */
export interface AccountantQuery {
  from: string;
  to: string;
  direction: AccountantDirection;
}
