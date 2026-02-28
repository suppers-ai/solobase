import { defineConfig } from '@playwright/test';
import path from 'path';

const cloudDir = path.resolve(__dirname, '..');
const binDir = path.resolve(cloudDir, 'e2e', '.bin');
const siteDir = path.resolve(cloudDir, '..', 'solobase-site');

export default defineConfig({
  testDir: './tests',
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: [['list']],
  timeout: 30_000,
  expect: {
    timeout: 10_000,
  },
  use: {
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  webServer: [
    {
      command: `mkdir -p ${binDir} && go build -o ${binDir}/mock-node ./cmd/mock-node && LISTEN_ADDR=:9090 NODE_SECRET=dev-secret ${binDir}/mock-node`,
      cwd: cloudDir,
      port: 9090,
      reuseExistingServer: !process.env.CI,
      timeout: 30_000,
    },
    {
      command: `mkdir -p ${binDir} && go build -o ${binDir}/solobase-cloud ./cmd && NODE_0="local-dev,http://localhost:9090,dev-secret,local,127.0.0.1" LISTEN_ADDR=:8080 API_SECRET=dev-secret BASE_URL=http://localhost:8080 DEV_MODE=1 ${binDir}/solobase-cloud`,
      cwd: cloudDir,
      port: 8080,
      reuseExistingServer: !process.env.CI,
      timeout: 30_000,
    },
    {
      command: 'npm install --prefer-offline && npm run dev -- --port 5173',
      cwd: siteDir,
      port: 5173,
      reuseExistingServer: !process.env.CI,
      timeout: 30_000,
    },
  ],
  projects: [
    {
      name: 'cloud',
      testMatch: ['cloud-dashboard.spec.ts', 'api-flow.spec.ts'],
      use: {
        baseURL: 'http://localhost:8080',
      },
    },
    {
      name: 'site',
      testMatch: ['marketing-site.spec.ts'],
      use: {
        baseURL: 'http://localhost:5173',
      },
    },
  ],
});
