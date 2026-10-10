import { describe, expect, it } from "vitest";
import { can, rolesUpTo, type Action } from "./roles";
import { ROLES, type Role } from "./types";

// Mirrors the roles matrix in docs/api/spaces.md.
const MATRIX: Record<Role, Action[]> = {
  accountant: ["read"],
  member: ["read", "write"],
  admin: ["read", "write", "settings", "viewAllTokens"],
  owner: ["read", "write", "settings", "viewAllTokens", "deleteSpace"],
};
const ACTIONS: Action[] = ["read", "write", "settings", "viewAllTokens", "deleteSpace"];

describe("can()", () => {
  it.each(ROLES)("%s may do exactly its row of the matrix", (role) => {
    expect(ACTIONS.filter((a) => can(role, a))).toEqual(MATRIX[role]);
  });

  it("allows nothing without a space role", () => {
    expect(ACTIONS.some((a) => can(null, a) || can(undefined, a))).toBe(false);
  });

  it("rolesUpTo lists the roles a token may carry", () => {
    expect(rolesUpTo("accountant")).toEqual(["accountant"]);
    expect(rolesUpTo("admin")).toEqual(["accountant", "member", "admin"]);
    expect(rolesUpTo("owner")).toEqual([...ROLES]);
  });
});
