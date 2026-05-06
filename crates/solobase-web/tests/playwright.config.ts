import { defineConfig, devices } from '@playwright/test';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const PORT = process.env.TEST_PORT ? parseInt(process.env.TEST_PORT) : 8080;

const HERE = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  testDir: './e2e',
  snapshotDir: '../../../.playwright-mcp',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: [['list']],
  timeout: 60_000,
  // Phase 5d Item A: log in once per test run instead of once per test. The
  // global-setup hook saves an `auth_token` cookie to ADMIN_STATE_PATH; admin
  // describes opt-in via `test.use({ storageState: ADMIN_STATE_PATH })`.
  // Anonymous describes inherit the config default (no storageState) and so
  // get a clean context.
  globalSetup: join(HERE, 'e2e/fixtures/global-setup.ts'),
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    serviceWorkers: 'allow',
  },
  projects: [
    { name: 'desktop-chrome', use: { ...devices['Desktop Chrome'] } },
  ],
});
