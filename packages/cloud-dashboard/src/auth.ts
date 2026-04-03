/** Auth state — simple module-level state (no signals dependency) */
import { api } from "./api";

export interface User {
  id: string;
  email: string;
  name?: string;
  roles?: string[];
}

let _user: User | null = null;
let _roles: string[] = [];

export function getUser(): User | null {
  return _user;
}
export function getRoles(): string[] {
  return _roles;
}
export function isAdmin(): boolean {
  return _roles.includes("admin");
}

export async function checkAuth(): Promise<boolean> {
  try {
    const data: any = await api.get("/b/auth/api/me");
    const user = data?.user || data;
    if (user?.id) {
      _user = user;
      _roles = Array.isArray(user.roles) ? user.roles : [];
      return true;
    }
  } catch {}
  _user = null;
  _roles = [];
  return false;
}

export function logout() {
  api.post("/b/auth/api/logout").catch(() => {});
  _user = null;
  _roles = [];
}
