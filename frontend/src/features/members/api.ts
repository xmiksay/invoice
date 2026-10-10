import { request } from "@/api/client";
import type { Role } from "@/features/spaces/types";
import type { AcceptInviteBody, Invite, InviteBody, InviteInfo, Member, SentInvite } from "./types";

const id = (value: string) => encodeURIComponent(value);

export const membersApi = {
  list: () => request<Member[]>("/api/members"),
  setRole: (userId: string, role: Role) => request<Member>(`/api/members/${id(userId)}`, { method: "PUT", body: { role } }),
  remove: (userId: string) => request<void>(`/api/members/${id(userId)}`, { method: "DELETE" }),
  leave: () => request<void>("/api/space/leave", { method: "POST" }),

  invites: () => request<Invite[]>("/api/invites"),
  invite: (body: InviteBody) => request<SentInvite>("/api/invites", { method: "POST", body }),
  resend: (inviteId: string) => request<SentInvite>(`/api/invites/${id(inviteId)}/resend`, { method: "POST" }),
  revoke: (inviteId: string) => request<void>(`/api/invites/${id(inviteId)}`, { method: "DELETE" }),

  // Signed-out routes: a 401 here is a wrong password, not an expired session.
  inviteInfo: (token: string) => request<InviteInfo>(`/api/invites/accept?token=${encodeURIComponent(token)}`, { quiet401: true }),
  accept: (body: AcceptInviteBody) => request<void>("/api/invites/accept", { method: "POST", body, quiet401: true }),
};
