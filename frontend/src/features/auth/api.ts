import { request } from "@/api/client";
import type { ChangePasswordBody, LoginBody, Me, RegisterBody } from "./types";

export const authApi = {
  // A 401 is the expected "not signed in" / "wrong credentials" answer for these two.
  me: () => request<Me>("/api/auth/me", { quiet401: true }),
  login: (body: LoginBody) => request<void>("/api/auth/login", { method: "POST", body, quiet401: true }),
  logout: () => request<void>("/api/auth/logout", { method: "POST", quiet401: true }),
  revokeOthers: () => request<void>("/api/auth/sessions/revoke-others", { method: "POST" }),
  register: (body: RegisterBody) => request<void>("/api/auth/register", { method: "POST", body }),
  verify: (token: string) => request<void>("/api/auth/verify", { method: "POST", body: { token } }),
  resendVerification: (email: string, locale: string) =>
    request<void>("/api/auth/verify/resend", { method: "POST", body: { email, locale } }),
  requestReset: (email: string, locale: string) =>
    request<void>("/api/auth/password-reset", { method: "POST", body: { email, locale } }),
  confirmReset: (token: string, password: string) =>
    request<void>("/api/auth/password-reset/confirm", { method: "POST", body: { token, password } }),
  changePassword: (body: ChangePasswordBody) => request<void>("/api/account/password", { method: "POST", body }),
};
