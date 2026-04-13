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
