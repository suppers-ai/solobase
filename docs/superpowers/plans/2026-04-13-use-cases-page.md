# Use Cases Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "Use Cases" page to solobase-site showcasing 12 use cases across 3 deployment model groups (Browser/WASM, Single Binary, Platform/Edge).

**Architecture:** New multi-page Vite entry point (`use-cases/index.html`) with a Preact page component. Uses existing site patterns — Header, Footer, Tailwind CSS. Use case data lives in a data file, rendered by a reusable card component. Group pills smooth-scroll to anchor sections.

**Tech Stack:** Preact, Vite (multi-page), Tailwind CSS

---

## File Structure

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `packages/solobase-site/use-cases/index.html` | HTML entry point for `/use-cases/` route |
| Create | `packages/solobase-site/src/pages/use-cases.jsx` | Page component — hero, pills, sections, CTA |
| Create | `packages/solobase-site/src/data/use-cases.js` | Use case data — titles, descriptions, groups, colors |
| Create | `packages/solobase-site/src/components/UseCaseCard.jsx` | Single use case card component |
| Modify | `packages/solobase-site/src/data/navigation.js` | Add "Use Cases" to `mainMenu` |
| Modify | `packages/solobase-site/vite.config.js` | Add `use-cases` entry to `rollupOptions.input` |
| Modify | `packages/solobase-site/tailwind.config.js` | Add `use-cases` HTML to `content` array |

---

### Task 1: Use Case Data

**Files:**
- Create: `packages/solobase-site/src/data/use-cases.js`

- [ ] **Step 1: Create the data file**

```js
export const groups = [
  {
    id: 'wasm',
    label: 'Browser / WASM',
    tag: 'WASM',
    heading: 'Run Anywhere — Backend in the Browser',
    subtitle: 'Your entire backend compiled to WebAssembly, running in a Service Worker. No server required.',
    color: {
      dot: '#7c3aed',
      tagBg: '#f5f3ff',
      tagText: '#7c3aed',
      pillBg: '#f5f3ff',
      pillBorder: '#7c3aed',
    },
  },
  {
    id: 'native',
    label: 'Single Binary',
    tag: 'Native',
    heading: 'Single Binary — Everything Included',
    subtitle: 'One binary, zero dependencies. Auth, database, storage, payments, admin panel — ready in seconds.',
    color: {
      dot: '#059669',
      tagBg: '#ecfdf5',
      tagText: '#059669',
      pillBg: '#ecfdf5',
      pillBorder: '#059669',
    },
  },
  {
    id: 'platform',
    label: 'Platform / Edge',
    tag: 'Platform',
    heading: 'Platform — Scale, Isolate, Trust',
    subtitle: 'Build services and apps on top of solobase. Multi-tenant, sandboxed, globally distributed.',
    color: {
      dot: '#d97706',
      tagBg: '#fffbeb',
      tagText: '#d97706',
      pillBg: '#fffbeb',
      pillBorder: '#d97706',
    },
  },
];

export const useCases = [
  // WASM
  {
    group: 'wasm',
    title: 'Per-PR Preview Environments',
    description: 'PR previews as static files. Reviewers click a link, the backend runs in their browser. No containers, no teardown.',
  },
  {
    group: 'wasm',
    title: 'Offline-First Apps',
    description: 'A real backend running locally with OPFS persistence. Auth, database, file storage — all working without connectivity.',
  },
  {
    group: 'wasm',
    title: 'Backend-in-the-Browser',
    description: 'Ship full-stack apps as static sites. Tutorials, interactive docs, product demos that prospects actually use.',
  },
  {
    group: 'wasm',
    title: 'Zero-Infrastructure E2E Testing',
    description: 'Playwright against solobase-web in headless Chrome. Fresh isolated backend per test. No Docker, no port conflicts.',
  },
  {
    group: 'wasm',
    title: 'Privacy-First / Data Sovereignty',
    description: 'User data never leaves the browser. GDPR and HIPAA-friendly by architecture, not just policy.',
  },
  // Native
  {
    group: 'native',
    title: 'Launch a SaaS Without the Stack',
    description: 'Skip the months of stitching together auth, payments, admin, and storage. One binary has it all. Focus on what makes your product different.',
  },
  {
    group: 'native',
    title: 'Free to Near-Free Hosting',
    description: "Host your backend as a free static file via WASM, or run your entire site on a single cheap worker. No managed databases, no monthly infrastructure bills.",
  },
  {
    group: 'native',
    title: 'Local-First Development',
    description: 'Download, run. Full backend on localhost in seconds. No Docker, no cloud account. Same code in dev and prod.',
  },
  {
    group: 'native',
    title: 'AI Agent Infrastructure',
    description: 'Agents spin up solobase, get auth + DB + storage + API instantly. No human provisioning. A2A messaging built in.',
  },
  // Platform
  {
    group: 'platform',
    title: 'Build Multi-Tenant Apps',
    description: "Give each customer their own isolated backend instance. Automatic provisioning, per-tenant config, full data separation — without managing infrastructure per customer.",
  },
  {
    group: 'platform',
    title: 'Safe Third-Party Extensions',
    description: "Let developers build extensions for your platform, sandboxed in WASM. No access to your secrets, no unauthorized network calls. Isolated by default.",
  },
  {
    group: 'platform',
    title: 'Deploy at the Edge',
    description: "Run your backend at edge locations worldwide. Your logic executes close to users, not in a single data center. Sub-50ms responses, globally.",
  },
];
```

- [ ] **Step 2: Commit**

```bash
git add packages/solobase-site/src/data/use-cases.js
git commit -m "feat(site): add use cases data"
```

---

### Task 2: UseCaseCard Component

**Files:**
- Create: `packages/solobase-site/src/components/UseCaseCard.jsx`

- [ ] **Step 1: Create the card component**

```jsx
export default function UseCaseCard({ title, description, tagLabel, color }) {
  return (
    <div
      class="bg-white border border-gray-200 rounded-xl p-6 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-md hover:border-gray-300"
    >
      <span
        class="inline-block text-xs font-semibold uppercase tracking-wide px-2.5 py-0.5 rounded-full mb-3"
        style={{ background: color.tagBg, color: color.tagText }}
      >
        {tagLabel}
      </span>
      <h3 class="text-lg font-semibold text-gray-800 mb-2">{title}</h3>
      <p class="text-gray-500 text-sm leading-relaxed">{description}</p>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/solobase-site/src/components/UseCaseCard.jsx
git commit -m "feat(site): add UseCaseCard component"
```

---

### Task 3: Use Cases Page Component

**Files:**
- Create: `packages/solobase-site/src/pages/use-cases.jsx`

- [ ] **Step 1: Create the page component**

```jsx
import { render } from 'preact';
import '../css/main.css';
import Header from '../components/Header';
import Footer from '../components/Footer';
import UseCaseCard from '../components/UseCaseCard';
import { groups, useCases } from '../data/use-cases';

function UseCasesPage() {
  return (
    <>
      <Header />
      <main>
        {/* Hero */}
        <section class="text-center py-16 sm:py-20 px-6 max-w-3xl mx-auto">
          <h1 class="text-responsive-xl font-bold text-gray-800 mb-4">
            Rethink what a{' '}
            <span style={{ color: 'var(--primary)' }}>backend</span> can do
          </h1>
          <p class="text-responsive-sm text-gray-500 max-w-xl mx-auto">
            A backend that runs in a browser tab, ships as a single binary, or
            scales across the globe. Same codebase, limitless possibilities.
          </p>
        </section>

        {/* Group pills */}
        <div class="flex justify-center gap-3 px-6 pb-12 flex-wrap">
          {groups.map((g) => (
            <a
              key={g.id}
              href={`#${g.id}`}
              class="px-5 py-2.5 rounded-full text-sm font-medium transition-all duration-200 hover:opacity-80"
              style={{
                background: g.color.pillBg,
                border: `2px solid ${g.color.pillBorder}`,
                color: g.color.pillBorder,
              }}
            >
              {g.label}
            </a>
          ))}
        </div>

        {/* Sections */}
        {groups.map((g, i) => {
          const items = useCases.filter((uc) => uc.group === g.id);
          return (
            <div key={g.id}>
              {i > 0 && (
                <hr class="border-t border-gray-200 max-w-6xl mx-auto" />
              )}
              <section
                id={g.id}
                class="max-w-6xl mx-auto px-6 py-12 scroll-mt-20"
              >
                <div class="flex items-center gap-3 mb-2">
                  <div
                    class="w-3 h-3 rounded-full"
                    style={{ background: g.color.dot }}
                  />
                  <h2 class="text-2xl sm:text-3xl font-bold text-gray-800">
                    {g.heading}
                  </h2>
                </div>
                <p class="text-gray-500 mb-8">{g.subtitle}</p>
                <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5">
                  {items.map((uc) => (
                    <UseCaseCard
                      key={uc.title}
                      title={uc.title}
                      description={uc.description}
                      tagLabel={g.tag}
                      color={g.color}
                    />
                  ))}
                </div>
              </section>
            </div>
          );
        })}

        {/* Bottom CTA */}
        <section class="text-center py-16 bg-gray-50">
          <h2 class="text-responsive-lg font-bold text-gray-800 mb-3">
            Ready to build?
          </h2>
          <p class="text-gray-500 text-lg mb-6">
            Same codebase runs everywhere. Start in the browser, deploy to the
            edge.
          </p>
          <div class="flex justify-center gap-3 flex-wrap">
            <a
              href="https://demo.solobase.dev"
              target="_blank"
              rel="noopener noreferrer"
              class="inline-flex items-center px-6 py-3 rounded-lg font-semibold text-white transition-all duration-200 hover:-translate-y-0.5 hover:shadow-lg"
              style={{ background: 'var(--primary)' }}
              onMouseOver={(e) =>
                (e.currentTarget.style.background = 'var(--primary-hover)')
              }
              onMouseOut={(e) =>
                (e.currentTarget.style.background = 'var(--primary)')
              }
            >
              Try in Browser
            </a>
            <a
              href="/"
              class="inline-flex items-center px-6 py-3 rounded-lg font-semibold text-white bg-gray-800 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-lg hover:bg-gray-700"
            >
              Download Binary
            </a>
          </div>
        </section>
      </main>
      <Footer />
    </>
  );
}

render(<UseCasesPage />, document.getElementById('app'));
```

- [ ] **Step 2: Commit**

```bash
git add packages/solobase-site/src/pages/use-cases.jsx
git commit -m "feat(site): add use cases page component"
```

---

### Task 4: HTML Entry Point

**Files:**
- Create: `packages/solobase-site/use-cases/index.html`

- [ ] **Step 1: Create the HTML entry**

```html
<!DOCTYPE html>
<html lang="en" class="scroll-smooth">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no">
  <meta name="theme-color" content="#2563eb">
  <title>Use Cases | Solobase</title>
  <meta name="description" content="Discover what you can build with Solobase — from browser-only apps to global edge platforms.">
  <meta name="author" content="Suppers Software Limited">
  <meta property="og:type" content="website">
  <meta property="og:title" content="Use Cases | Solobase">
  <meta property="og:description" content="Discover what you can build with Solobase — from browser-only apps to global edge platforms.">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="Use Cases | Solobase">
  <meta name="twitter:description" content="Discover what you can build with Solobase — from browser-only apps to global edge platforms.">
  <link rel="icon" type="image/x-icon" href="/favicon.ico">
  <link rel="stylesheet" href="/fonts/itim.css">
</head>
<body class="bg-gray-50 text-gray-900">
  <div id="app"></div>
  <script type="module" src="/src/pages/use-cases.jsx"></script>
</body>
</html>
```

- [ ] **Step 2: Commit**

```bash
git add packages/solobase-site/use-cases/index.html
git commit -m "feat(site): add use-cases HTML entry point"
```

---

### Task 5: Wire Up Navigation and Build Config

**Files:**
- Modify: `packages/solobase-site/src/data/navigation.js:11-16`
- Modify: `packages/solobase-site/vite.config.js:30-36`
- Modify: `packages/solobase-site/tailwind.config.js:3-8`

- [ ] **Step 1: Add "Use Cases" to navigation**

In `packages/solobase-site/src/data/navigation.js`, add the Use Cases entry to `mainMenu` after "Home":

```js
export const mainMenu = [
  { name: "Home", url: "/" },
  { name: "Use Cases", url: "/use-cases/" },
  { name: "Pricing", url: "/pricing/" },
  { name: "Docs", url: "/docs/" },
  { name: "Demo", url: "https://demo.solobase.dev", external: true },
  {
    name: "Sign In",
    url: "https://cloud.solobase.dev/b/auth/login",
    external: true,
  },
];
```

- [ ] **Step 2: Add Vite build entry**

In `packages/solobase-site/vite.config.js`, add `use-cases` to `rollupOptions.input`:

```js
rollupOptions: {
  input: {
    main: resolve(__dirname, 'index.html'),
    pricing: resolve(__dirname, 'pricing/index.html'),
    'use-cases': resolve(__dirname, 'use-cases/index.html'),
  },
},
```

- [ ] **Step 3: Add to Tailwind content scan**

In `packages/solobase-site/tailwind.config.js`, add `use-cases` HTML to the `content` array:

```js
content: [
  './index.html',
  './pricing/**/*.html',
  './use-cases/**/*.html',
  './docs/**/*.html',
  './src/**/*.{js,jsx}',
],
```

- [ ] **Step 4: Commit**

```bash
git add packages/solobase-site/src/data/navigation.js packages/solobase-site/vite.config.js packages/solobase-site/tailwind.config.js
git commit -m "feat(site): wire up use-cases route, nav, and build config"
```

---

### Task 6: Verify

- [ ] **Step 1: Install dependencies and start dev server**

```bash
cd packages/solobase-site && npm install && npm run dev
```

- [ ] **Step 2: Verify in browser**

Open the dev server URL and check:
- Home page header shows "Use Cases" link between "Home" and "Pricing"
- Clicking "Use Cases" navigates to `/use-cases/`
- Hero section renders with "Rethink what a backend can do"
- Three colored pills render and scroll to their sections on click
- All 12 use case cards render across 3 sections with correct tags/colors
- Cards hover effect works (slight lift + shadow)
- Bottom CTA "Try in Browser" links to `https://demo.solobase.dev`
- Footer renders correctly
- Page is responsive on mobile (cards stack, pills wrap)

- [ ] **Step 3: Verify build**

```bash
cd packages/solobase-site && npm run build
```

Confirm `dist/site/use-cases/index.html` exists in the output.

- [ ] **Step 4: Commit any fixes if needed**
