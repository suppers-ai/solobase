import { test, expect } from '@playwright/test';
import { loginAsAdmin } from './fixtures/auth';

const ANON_ROUTES = [
  { path: '/b/auth/login', name: 'auth-login' },
  { path: '/b/auth/signup', name: 'auth-signup' },
  { path: '/totally-not-a-thing', name: 'status-404' },
];

const ADMIN_ROUTES = [
  { path: '/', name: 'root-redirect' },
  { path: '/b/admin/', name: 'admin-dashboard' },
  { path: '/b/admin/users', name: 'admin-users' },
  { path: '/b/admin/blocks', name: 'admin-blocks' },
  { path: '/b/admin/database', name: 'admin-database' },
  { path: '/b/admin/storage', name: 'admin-storage' },
  { path: '/b/admin/logs', name: 'admin-logs' },
  { path: '/b/admin/email', name: 'admin-email' },
  { path: '/b/admin/network', name: 'admin-network' },
  { path: '/b/admin/variables', name: 'admin-variables' },
  { path: '/b/admin/permissions', name: 'admin-permissions' },
  { path: '/b/auth/dashboard', name: 'portal-dashboard' },
  { path: '/b/auth/orgs', name: 'portal-orgs' },
  { path: '/b/userportal/profile', name: 'portal-profile' },
  { path: '/b/userportal/sessions', name: 'portal-sessions' },
  { path: '/b/userportal/security', name: 'portal-security' },
  { path: '/b/products/', name: 'portal-products' },
];

const COMMON_OPTS = {
  fullPage: true as const,
  maxDiffPixelRatio: 0.01,
  // Mask elements that vary per render (timestamps, counts, generated IDs).
  // Tests can override per-route if needed.
};

test.describe('visual baseline — anonymous', () => {
  for (const r of ANON_ROUTES) {
    test(`anon ${r.name}`, async ({ page }) => {
      await page.goto(r.path, { waitUntil: 'networkidle' });
      await expect(page).toHaveScreenshot(`anon-${r.name}.png`, COMMON_OPTS);
    });
  }
});

test.describe('visual baseline — admin', () => {
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);
  });
  for (const r of ADMIN_ROUTES) {
    test(`admin ${r.name}`, async ({ page }) => {
      await page.goto(r.path, { waitUntil: 'networkidle' });
      await expect(page).toHaveScreenshot(`admin-${r.name}.png`, {
        ...COMMON_OPTS,
        // Mask relative timestamps anywhere they appear.
        mask: [page.locator('[data-relative-time], .relative-time, time')],
      });
    });
  }
});

// ===== Phase 4 PR-3: 375px mobile pass =====

const MOBILE_VIEWPORT = { width: 375, height: 812 } as const;

const MOBILE_ANON_ROUTES = [
  { path: '/b/auth/login', name: 'auth-login' },
  { path: '/b/auth/signup', name: 'auth-signup' },
];

const MOBILE_ADMIN_ROUTES = [
  // Portal pages — primary mobile audience. Admin is desktop-first per
  // master spec; the hint banner inside is the explicit accommodation.
  // Admin pages aren't mobile-snapshotted because the dashboard's
  // "Recent Errors" table is fed by the request_log accumulated during
  // the rest of the baseline run, so its content + width drift between
  // local and CI.
  { path: '/b/auth/dashboard', name: 'portal-dashboard' },
  { path: '/b/auth/orgs', name: 'portal-orgs' },
  { path: '/b/userportal/profile', name: 'portal-profile' },
  { path: '/b/userportal/sessions', name: 'portal-sessions' },
  { path: '/b/userportal/security', name: 'portal-security' },
  { path: '/b/products/', name: 'portal-products' },
];

test.describe('visual baseline mobile — anonymous (375px)', () => {
  for (const r of MOBILE_ANON_ROUTES) {
    test(`anon-mobile ${r.name}`, async ({ page }) => {
      await page.setViewportSize(MOBILE_VIEWPORT);
      await page.goto(r.path, { waitUntil: 'networkidle' });
      await expect(page).toHaveScreenshot(`anon-${r.name}-mobile.png`, COMMON_OPTS);
    });
  }
});

test.describe('visual baseline mobile — admin (375px)', () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize(MOBILE_VIEWPORT);
    await loginAsAdmin(page);
  });
  for (const r of MOBILE_ADMIN_ROUTES) {
    test(`admin-mobile ${r.name}`, async ({ page }) => {
      await page.setViewportSize(MOBILE_VIEWPORT);
      await page.goto(r.path, { waitUntil: 'networkidle' });
      await expect(page).toHaveScreenshot(`admin-${r.name}-mobile.png`, {
        ...COMMON_OPTS,
        mask: [page.locator('[data-relative-time], .relative-time, time')],
      });
    });
  }
});
