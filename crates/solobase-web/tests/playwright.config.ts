import { defineConfig, devices } from '@playwright/test';

const PORT = process.env.TEST_PORT ? parseInt(process.env.TEST_PORT) : 8080;

export default defineConfig({
  testDir: './e2e',
  snapshotDir: '../../../.playwright-mcp',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: [['list']],
  timeout: 60_000,
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    serviceWorkers: 'allow',
  },
  projects: [
    {
      name: 'desktop-chrome',
      // `channel: 'chromium'` forces the full Chrome-for-Testing binary in
      // Chrome's new headless mode. Playwright's default for headless runs
      // is `chrome-headless-shell`, which is a smaller alternate binary
      // that diverges from full Chromium on some web-platform features —
      // notably for this repo, it triggers an Emscripten "null function"
      // error from sql.js's `new SQL.Database()` constructor on first
      // wasm instantiation in a Service Worker. The full Chromium build
      // (which Playwright also installs via `npx playwright install
      // chromium --with-deps`) doesn't have that bug.
      use: { ...devices['Desktop Chrome'], channel: 'chromium' },
    },
  ],
});
