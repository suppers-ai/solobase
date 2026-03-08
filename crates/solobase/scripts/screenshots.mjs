import { chromium } from '@playwright/test';

const BASE = 'http://127.0.0.1:8090';
const DIR = '/tmp/solobase-screenshots';

async function main() {
  const browser = await chromium.launch();
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });

  // Helper
  async function shot(page, name) {
    await page.screenshot({ path: `${DIR}/${name}.png`, fullPage: true });
    console.log(`  saved ${name}.png`);
  }

  // 1. Landing page
  const page = await context.newPage();
  await page.goto(BASE);
  await page.waitForLoadState('networkidle');
  await shot(page, '01-landing');

  // 2. Login page
  await page.goto(`${BASE}/blocks/auth/frontend/`);
  await page.waitForLoadState('networkidle');
  await shot(page, '02-login');

  // 3. Sign up a dev user
  const signupRes = await page.request.post(`${BASE}/auth/signup`, {
    data: { email: 'screenshot@test.com', password: 'TestPass1234', name: 'Test User' },
  });
  const signupBody = await signupRes.json();
  if (!signupBody.access_token) console.error('Signup response:', JSON.stringify(signupBody));
  const devToken = signupBody.access_token;

  // Also login as admin
  const adminRes = await page.request.post(`${BASE}/auth/login`, {
    data: { email: 'admin@example.com', password: 'admin123' },
  });
  const adminBody = await adminRes.json();
  if (!adminBody.access_token) console.error('Admin login response:', JSON.stringify(adminBody));
  const adminToken = adminBody.access_token;

  // Helper: go to dashboard with auth
  async function goToDashboard(token, hash = 'overview') {
    const p = await context.newPage();
    await p.context().addCookies([{
      name: 'auth_token', value: token,
      domain: '127.0.0.1', path: '/',
      httpOnly: true, sameSite: 'Lax',
    }]);
    await p.goto(`${BASE}/blocks/dashboard/frontend/#${hash}`);
    await p.waitForLoadState('networkidle');
    await p.waitForTimeout(1000);
    return p;
  }

  // 4. Dashboard overview
  let p = await goToDashboard(devToken, 'overview');
  await shot(p, '03-dashboard-overview');
  await p.close();

  // 5. Plans tab
  p = await goToDashboard(devToken, 'plans');
  await shot(p, '04-dashboard-plans');
  await p.close();

  // 6. Deployments tab
  p = await goToDashboard(devToken, 'deployments');
  await shot(p, '05-dashboard-deployments');
  await p.close();

  // 7. API Keys tab
  p = await goToDashboard(devToken, 'api-keys');
  await shot(p, '06-dashboard-apikeys');
  await p.close();

  // 8. Settings tab
  p = await goToDashboard(devToken, 'settings');
  await shot(p, '07-dashboard-settings');
  await p.close();

  // 9. Admin panel
  const adminPage = await context.newPage();
  await adminPage.context().addCookies([{
    name: 'auth_token', value: adminToken,
    domain: '127.0.0.1', path: '/',
    httpOnly: true, sameSite: 'Lax',
  }]);
  await adminPage.goto(`${BASE}/blocks/admin/frontend/`);
  await adminPage.waitForLoadState('networkidle');
  await adminPage.waitForTimeout(1000);
  await shot(adminPage, '08-admin');
  await adminPage.close();

  // 10. Docs
  const docsPage = await context.newPage();
  await docsPage.goto(`${BASE}/docs/`);
  await docsPage.waitForLoadState('networkidle');
  await shot(docsPage, '09-docs');
  await docsPage.close();

  // 11. Mock stripe checkout page (optional)
  try {
    const stripePage = await context.newPage();
    await stripePage.goto('http://127.0.0.1:12111/checkout/fake-session', { timeout: 3000 });
    await stripePage.waitForLoadState('networkidle');
    await shot(stripePage, '10-stripe-mock');
    await stripePage.close();
  } catch { console.log('  skipped stripe mock (not running)'); }

  await browser.close();
  console.log(`\nAll screenshots saved to ${DIR}/`);
}

main().catch(e => { console.error(e); process.exit(1); });
