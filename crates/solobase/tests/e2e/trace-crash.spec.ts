import { test, expect } from '@playwright/test';

const ADMIN_EMAIL = 'admin@example.com';
const ADMIN_PASSWORD = 'admin123';

async function login(page) {
  const response = await page.request.post('/api/auth/login', {
    data: { email: ADMIN_EMAIL, password: ADMIN_PASSWORD },
  });
  expect(response.status()).toBe(200);
}

test('trace createElementNS crash on dashboard', async ({ page }) => {
  await login(page);

  // Inject monkey-patch BEFORE page loads
  await page.addInitScript(() => {
    const origCreateElementNS = document.createElementNS.bind(document);
    document.createElementNS = function(ns: string, qn: string, ...args: any[]) {
      if (!qn || qn === '') {
        // Capture the full call stack
        const err = new Error(`createElementNS called with empty qualifiedName! ns=${ns}`);
        console.error('CRASH_TRACE:', err.stack);
        // Also log what the global Preact state looks like
        console.error('CRASH_ARGS:', JSON.stringify({ ns, qn, argsLen: args.length }));
      }
      return origCreateElementNS(ns, qn, ...args);
    };
  });

  // Now navigate
  const messages: string[] = [];
  page.on('console', msg => {
    if (msg.text().includes('CRASH_')) {
      messages.push(msg.text());
    }
  });
  page.on('pageerror', err => {
    messages.push(`[PAGEERROR] ${err.message}`);
  });

  await page.goto('/admin');
  await page.waitForTimeout(3000);

  console.log('=== Crash Trace Messages ===');
  for (const m of messages) {
    console.log(m);
  }

  // Try to get the source-mapped location using page.evaluate
  const debugInfo = await page.evaluate(() => {
    return {
      title: document.title,
      appContent: document.getElementById('app')?.innerHTML?.substring(0, 500) || 'EMPTY',
      scripts: Array.from(document.querySelectorAll('script')).map(s => s.src || 'inline'),
    };
  });
  console.log('Page info:', JSON.stringify(debugInfo, null, 2));
});

test('trace crash on simple admin page (users)', async ({ page }) => {
  await login(page);

  await page.addInitScript(() => {
    const origCreateElementNS = document.createElementNS.bind(document);
    document.createElementNS = function(ns: string, qn: string, ...args: any[]) {
      if (!qn || qn === '') {
        console.error('CRASH_TRACE:', new Error(`Empty qualifiedName! ns=${ns}`).stack);
      }
      return origCreateElementNS(ns, qn, ...args);
    };
  });

  const messages: string[] = [];
  page.on('console', msg => {
    if (msg.text().includes('CRASH_')) {
      messages.push(msg.text());
    }
  });
  page.on('pageerror', err => {
    messages.push(`[PAGEERROR] ${err.message}`);
  });

  await page.goto('/admin/users');
  await page.waitForTimeout(3000);

  console.log('=== Users Page Crash Trace ===');
  for (const m of messages) {
    console.log(m);
  }
});

test('check if login page works (control test)', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', err => {
    errors.push(err.message);
  });

  await page.goto('/admin/login');
  await page.waitForTimeout(2000);

  const appContent = await page.locator('#app').innerHTML();
  console.log(`Login page #app content length: ${appContent.length}`);
  console.log(`Login page errors: ${errors.length}`);
  console.log(`Login page rendered: ${appContent.length > 50 ? 'YES' : 'NO'}`);
});
