import { test, expect } from "@playwright/test";

test.describe("Health & System", () => {
  test("GET /health returns ok", async ({ request }) => {
    const res = await request.get("/health");
    expect(res.ok()).toBeTruthy();
    expect(await res.json()).toEqual({ status: "ok" });
  });
});

async function signupAndLogin(request: any) {
  const email = `e2e-${Date.now()}-${Math.random().toString(36).slice(2, 8)}@test.com`;
  const password = "TestPass1234";
  const signup = await request.post("/b/auth/api/signup", {
    data: { email, password },
  });
  const body = await signup.json();
  return { email, password, token: body.access_token, userId: body.user?.id };
}

test.describe("Auth", () => {
  test("POST /b/auth/api/signup creates user and returns tokens", async ({
    request,
  }) => {
    const { email, token } = await signupAndLogin(request);
    expect(token).toBeTruthy();
  });

  test("POST /b/auth/api/signup rejects duplicate email", async ({
    request,
  }) => {
    const { email, password } = await signupAndLogin(request);
    const res = await request.post("/b/auth/api/signup", {
      data: { email, password },
    });
    expect(res.status()).toBe(409);
  });

  test("POST /b/auth/api/login returns tokens", async ({ request }) => {
    const { email, password } = await signupAndLogin(request);
    const res = await request.post("/b/auth/api/login", {
      data: { email, password },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body).toHaveProperty("access_token");
    expect(body.user.email).toBe(email);
  });

  test("GET /b/auth/api/me returns user profile", async ({ request }) => {
    const { email, token } = await signupAndLogin(request);
    const res = await request.get("/b/auth/api/me", {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.user.email).toBe(email);
  });
});

test.describe("Legal Pages", () => {
  test("GET /b/legalpages/terms returns HTML", async ({ request }) => {
    const res = await request.get("/b/legalpages/terms");
    expect(res.ok()).toBeTruthy();
    const ct = res.headers()["content-type"];
    expect(ct).toContain("text/html");
  });

  test("GET /b/legalpages/privacy returns HTML", async ({ request }) => {
    const res = await request.get("/b/legalpages/privacy");
    expect(res.ok()).toBeTruthy();
    const ct = res.headers()["content-type"];
    expect(ct).toContain("text/html");
  });
});

test.describe("Frontend SPA", () => {
  test("GET / serves index.html", async ({ request }) => {
    const res = await request.get("/");
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain("Solobase");
  });

  test("unknown path serves SPA fallback", async ({ request }) => {
    const res = await request.get("/some/unknown/path");
    expect(res.ok()).toBeTruthy();
    const html = await res.text();
    expect(html).toContain("Solobase");
  });
});
