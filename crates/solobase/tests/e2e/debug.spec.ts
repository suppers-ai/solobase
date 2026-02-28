import { test, expect } from '@playwright/test';

const ADMIN_EMAIL = 'admin@example.com';
const ADMIN_PASSWORD = 'admin123';

async function login(page) {
  const response = await page.request.post('/api/auth/login', {
    data: { email: ADMIN_EMAIL, password: ADMIN_PASSWORD },
  });
  expect(response.status()).toBe(200);
}

test('debug dashboard rendering', async ({ page }) => {
  // Capture ALL console messages
  const messages: string[] = [];
  page.on('console', msg => {
    messages.push(`[${msg.type()}] ${msg.text()}`);
  });
  page.on('pageerror', err => {
    messages.push(`[PAGEERROR] ${err.message}\n${err.stack}`);
  });

  await login(page);
  await page.goto('/admin');
  await page.waitForTimeout(3000);

  // Print all console messages
  console.log('=== Console Messages ===');
  for (const m of messages) {
    console.log(m);
  }

  // Check the HTML source
  const html = await page.content();
  console.log('\n=== HTML (first 2000 chars) ===');
  console.log(html.substring(0, 2000));

  // Check the #app div
  const appDiv = page.locator('#app');
  const appHTML = await appDiv.innerHTML();
  console.log('\n=== #app innerHTML (first 1000 chars) ===');
  console.log(appHTML.substring(0, 1000));

  await page.screenshot({ path: 'test-results/debug-dashboard.png', fullPage: true });
});

test('debug waffle page rendering', async ({ page }) => {
  const messages: string[] = [];
  page.on('console', msg => {
    messages.push(`[${msg.type()}] ${msg.text()}`);
  });
  page.on('pageerror', err => {
    messages.push(`[PAGEERROR] ${err.message}\n${err.stack}`);
  });

  await login(page);
  await page.goto('/admin/waffle');
  await page.waitForTimeout(3000);

  console.log('=== Console Messages ===');
  for (const m of messages) {
    console.log(m);
  }

  const appDiv = page.locator('#app');
  const appHTML = await appDiv.innerHTML();
  console.log('\n=== #app innerHTML (first 1000 chars) ===');
  console.log(appHTML.substring(0, 1000));

  // Check blocks API specifically
  const blocksResp = await page.request.get('/api/admin/waffle/blocks');
  console.log(`\nBlocks API status: ${blocksResp.status()}`);
  const blocksText = await blocksResp.text();
  console.log(`Blocks response (first 500 chars): ${blocksText.substring(0, 500)}`);

  await page.screenshot({ path: 'test-results/debug-waffle.png', fullPage: true });
});
