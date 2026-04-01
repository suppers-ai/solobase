/** Lightweight API client — replaces @solobase/ui api */

const TOKEN_KEY = "auth_token";

function getToken(): string | null {
  // Cookie-based auth — the HttpOnly cookie is sent automatically.
  // This helper exists for any future header-based auth needs.
  return null;
}

async function request<T = any>(
  method: string,
  path: string,
  body?: any,
): Promise<T> {
  const headers: Record<string, string> = { Accept: "application/json" };
  if (body) headers["Content-Type"] = "application/json";

  const res = await fetch(path, {
    method,
    headers,
    credentials: "same-origin",
    body: body ? JSON.stringify(body) : undefined,
  });

  if (res.status === 401) {
    window.location.href =
      "/b/auth/login?redirect=" + encodeURIComponent(window.location.href);
    throw new Error("Unauthorized");
  }

  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    const msg =
      data?.error?.message ||
      data?.message ||
      data?.error ||
      `Request failed (${res.status})`;
    throw new Error(msg);
  }
  return data as T;
}

export const api = {
  get: <T = any>(path: string) => request<T>("GET", path),
  post: <T = any>(path: string, body?: any) => request<T>("POST", path, body),
  put: <T = any>(path: string, body?: any) => request<T>("PUT", path, body),
  delete: <T = any>(path: string) => request<T>("DELETE", path),
};
