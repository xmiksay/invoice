import { can } from "@/features/spaces/roles";
import { ROLES, type Role } from "@/features/spaces/types";

/** Roles the caller may grant (invite with or assign): only an owner grants `owner`. */
export function grantableRoles(caller: Role | null): Role[] {
  if (!can(caller, "settings")) return [];
  return caller === "owner" ? [...ROLES] : ROLES.filter((r) => r !== "owner");
}

/**
 * Roles the caller may set on a member who currently holds `target` (docs/api/members.md);
 * empty = the member is out of the caller's reach (an owner seen by an admin, the last owner),
 * which also means the caller may not remove them.
 */
export function assignableRoles(caller: Role | null, target: Role, ownersCount: number): Role[] {
  const grantable = grantableRoles(caller);
  if (!grantable.includes(target)) return [];
  if (target === "owner" && ownersCount <= 1) return [];
  return grantable;
}
