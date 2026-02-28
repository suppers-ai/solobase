import { test, expect } from '@playwright/test';

const ADMIN_EMAIL = 'admin@example.com';
const ADMIN_PASSWORD = 'admin123';

async function login(page) {
  const response = await page.request.post('/api/auth/login', {
    data: { email: ADMIN_EMAIL, password: ADMIN_PASSWORD },
  });
  expect(response.status()).toBe(200);
}

test('deep trace: find the empty VNode type', async ({ page }) => {
  await login(page);

  // Intercept Preact's createElement to catch vnodes with empty type
  await page.addInitScript(() => {
    // Intercept createElementNS to get full context
    const origCreateElementNS = document.createElementNS.bind(document);
    (document as any).__createElementNS_count = 0;
    document.createElementNS = function(ns: string, qn: string, ...args: any[]) {
      (document as any).__createElementNS_count++;
      if (!qn || qn === '') {
        // Walk back up the call stack through Preact internals
        // Log the Preact internal state
        const preact = (window as any).__PREACT_DEVTOOLS__?.getRoots?.() || 'no devtools';

        // Try to find the component being rendered by examining the stack
        const stack = new Error().stack || '';
        console.error('VNODE_CRASH: Empty element creation detected');
        console.error('VNODE_STACK:', stack);
        console.error('VNODE_COUNT: Elements created before crash:', (document as any).__createElementNS_count);

        // Try to get info from Preact's internal state
        // Preact stores current component in __c during rendering
        try {
          // Look for any global Preact hooks
          const w = window as any;
          if (w.__PREACT_DEVTOOLS__) {
            console.error('VNODE_DEVTOOLS:', JSON.stringify(w.__PREACT_DEVTOOLS__));
          }
        } catch (e) {
          console.error('VNODE_DEVTOOLS_ERROR:', e);
        }
      }
      return origCreateElementNS(ns, qn, ...args);
    };

    // Also intercept h/createElement to find vnodes with empty type
    // We need to hook into the module system. One approach: intercept at the h() level.
    // Since Preact is bundled, we can override h at the options level.
    const origDefineProperty = Object.defineProperty;
    let preactOptionsHooked = false;

    // Monitor for Preact's __b (before diff) hook
    const checkPreact = setInterval(() => {
      const w = window as any;
      // Preact exposes options on the preact module. In the bundled version,
      // we need to find it. One approach: __PREACT_DEVTOOLS__ or the options object.

      // Try to hook into Preact's options.__b (before diff)
      // This is internal but standard in Preact's architecture
      if (w.__b && !preactOptionsHooked) {
        preactOptionsHooked = true;
        const origB = w.__b;
        w.__b = function(vnode: any) {
          if (vnode && typeof vnode.type === 'string' && vnode.type === '') {
            console.error('VNODE_EMPTY_TYPE: Found VNode with empty string type!');
            console.error('VNODE_PROPS:', JSON.stringify(vnode.props || {}));
            console.error('VNODE_KEY:', vnode.key);
            console.error('VNODE_PARENT:', vnode.__?.type?.name || vnode.__?.type || 'unknown');
          }
          if (origB) origB(vnode);
        };
        clearInterval(checkPreact);
      }
    }, 1);
  });

  const messages: string[] = [];
  page.on('console', msg => {
    const text = msg.text();
    if (text.includes('VNODE_') || text.includes('CRASH')) {
      messages.push(text);
    }
  });
  page.on('pageerror', err => {
    messages.push(`[PAGEERROR] ${err.message}`);
  });

  await page.goto('/admin');
  await page.waitForTimeout(3000);

  console.log('=== VNode Trace Messages ===');
  for (const m of messages) {
    console.log(m);
  }
});

test('test: use vite dev server instead of built files', async ({ page }) => {
  // Check if vite dev server is running on :5173
  await login(page);

  // Try to get the source-mapped content from the built JS
  // Let's get the dashboard chunk and look at line 146
  const dashboardJS = await page.request.get('/assets/dashboard-QU3rDlj8.js');
  const content = await dashboardJS.text();
  const lines = content.split('\n');
  console.log(`Dashboard chunk: ${lines.length} lines`);
  if (lines.length >= 146) {
    console.log(`Line 146 (first 300 chars): ${lines[145].substring(0, 300)}`);
  }
  // Also look at the lines around it
  for (let i = Math.max(0, 140); i < Math.min(lines.length, 150); i++) {
    console.log(`Line ${i+1}: ${lines[i].substring(0, 200)}`);
  }
});
