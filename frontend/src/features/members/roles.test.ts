import { describe, expect, it } from "vitest";
import { ROLES } from "@/features/spaces/types";
import { assignableRoles, grantableRoles } from "./roles";

const BELOW_OWNER = ["accountant", "member", "admin"];

describe("grantableRoles()", () => {
  it("gives nothing below admin, no owner to an admin, everything to an owner", () => {
    expect(grantableRoles(null)).toEqual([]);
    expect(grantableRoles("accountant")).toEqual([]);
    expect(grantableRoles("member")).toEqual([]);
    expect(grantableRoles("admin")).toEqual(BELOW_OWNER);
    expect(grantableRoles("owner")).toEqual([...ROLES]);
  });
});

describe("assignableRoles()", () => {
  it("lets nobody below admin change anyone", () => {
    for (const target of ROLES) {
      expect(assignableRoles("member", target, 2)).toEqual([]);
      expect(assignableRoles("accountant", target, 2)).toEqual([]);
    }
  });

  it("lets an admin move non-owners between the roles below owner", () => {
    for (const target of BELOW_OWNER) expect(assignableRoles("admin", target as never, 1)).toEqual(BELOW_OWNER);
  });

  it("keeps owners out of an admin's reach", () => {
    expect(assignableRoles("admin", "owner", 1)).toEqual([]);
    expect(assignableRoles("admin", "owner", 3)).toEqual([]);
  });

  it("lets an owner grant every role, including owner", () => {
    expect(assignableRoles("owner", "member", 1)).toEqual([...ROLES]);
    expect(assignableRoles("owner", "owner", 2)).toEqual([...ROLES]);
  });

  it("never lets the last owner be demoted", () => {
    expect(assignableRoles("owner", "owner", 1)).toEqual([]);
  });
});
