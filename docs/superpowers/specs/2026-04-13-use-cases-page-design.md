# Use Cases Page — Design Spec

## Overview

A new "Use Cases" page for solobase-site that showcases what developers can build with solobase across its three deployment models. The page is inspirational but grounded — each use case leads with a vision and keeps copy concise.

## Page Structure

Single scrollable page at `/use-cases`. Added to the site header navigation.

### Hero

- **Heading**: "Rethink what a backend can do"
- **Subtitle**: "A backend that runs in a browser tab, ships as a single binary, or scales across the globe. Same codebase, limitless possibilities."

### Group Navigation Pills

Three pill buttons that scroll to their respective sections:
- Browser / WASM (purple: #7c3aed)
- Single Binary (green: #059669)
- Platform / Edge (amber: #d97706)

No use case counts in the pills — the list will grow over time.

### Sections

Each section has:
- Colored dot + heading
- One-line subtitle
- Card grid (responsive, `minmax(320px, 1fr)`)

Cards have:
- Colored tag (WASM / Native / Platform)
- Title
- 1-2 sentence description — concise, straight to the point

No "groundbreaking because" lines. The cards speak for themselves.

---

## Section 1: Browser / WASM

**Heading**: Run Anywhere — Backend in the Browser
**Subtitle**: Your entire backend compiled to WebAssembly, running in a Service Worker. No server required.
**Color**: Purple (#7c3aed)

### Cards

**Per-PR Preview Environments**
PR previews as static files. Reviewers click a link, the backend runs in their browser. No containers, no teardown.

**Offline-First Apps**
A real backend running locally with OPFS persistence. Auth, database, file storage — all working without connectivity.

**Backend-in-the-Browser**
Ship full-stack apps as static sites. Tutorials, interactive docs, product demos that prospects actually use.

**Zero-Infrastructure E2E Testing**
Playwright against solobase-web in headless Chrome. Fresh isolated backend per test. No Docker, no port conflicts.

**Privacy-First / Data Sovereignty**
User data never leaves the browser. GDPR and HIPAA-friendly by architecture, not just policy.

---

## Section 2: Single Binary

**Heading**: Single Binary — Everything Included
**Subtitle**: One binary, zero dependencies. Auth, database, storage, payments, admin panel — ready in seconds.
**Color**: Green (#059669)

### Cards

**Launch a SaaS Without the Stack**
Skip the months of stitching together auth, payments, admin, and storage. One binary has it all. Focus on what makes your product different.

**Free to Near-Free Hosting**
Host your backend as a free static file via WASM, or run your entire site on a single cheap worker. No managed databases, no monthly infrastructure bills.

**Local-First Development**
Download, run. Full backend on localhost in seconds. No Docker, no cloud account. Same code in dev and prod.

**AI Agent Infrastructure**
Agents spin up solobase, get auth + DB + storage + API instantly. No human provisioning. A2A messaging built in.

---

## Section 3: Platform / Edge

**Heading**: Platform — Scale, Isolate, Trust
**Subtitle**: Build services and apps on top of solobase. Multi-tenant, sandboxed, globally distributed.
**Color**: Amber (#d97706)

### Cards

**Build Multi-Tenant Apps**
Give each customer their own isolated backend instance. Automatic provisioning, per-tenant config, full data separation — without managing infrastructure per customer.

**Safe Third-Party Extensions**
Let developers build extensions for your platform, sandboxed in WASM. No access to your secrets, no unauthorized network calls. Isolated by default.

**Deploy at the Edge**
Run your backend at edge locations worldwide. Your logic executes close to users, not in a single data center. Sub-50ms responses, globally.

---

## Bottom CTA

- **Heading**: "Ready to build?"
- **Subtitle**: "Same codebase runs everywhere. Start in the browser, deploy to the edge."
- **Buttons**: "Try in Browser" (primary orange) + "Download Binary" (dark secondary)

## Technical Details

The site uses multi-page Vite builds (separate HTML entry points per route, no client-side router).

- **Page component**: `packages/solobase-site/src/pages/use-cases.jsx`
- **HTML entry**: `packages/solobase-site/use-cases/index.html` (new, mirrors `pricing/index.html` pattern)
- **Vite config**: Add `use-cases` entry to `rollupOptions.input` in `vite.config.js`
- **Navigation**: Add `{ name: "Use Cases", url: "/use-cases/" }` to `mainMenu` in `src/data/navigation.js`
- **Styling**: Tailwind CSS, matching existing site conventions (Itim font, #fe6627 orange accent, card borders #e5e7eb, rounded-xl)
- **Responsive**: Cards grid collapses to single column on mobile. Pills wrap on small screens.
- **Interactions**: Group pills smooth-scroll to anchor sections (`scroll-smooth` is already on the `<html>` tag). Cards have hover lift effect (translateY -2px, subtle box shadow).

## Visual Reference

Mockup saved at `.superpowers/brainstorm/116825-1776076883/content/page-mockup-v4.html`
