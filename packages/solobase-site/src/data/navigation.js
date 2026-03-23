export const siteConfig = {
  title: 'Solobase - Modern Admin Dashboard',
  description: 'Open source backend in a single binary',
  author: 'Suppers Software Limited',
  logo: '/images/logo_long.png',
  demoUrl: 'https://solobase.dev/',
  githubUrl: 'https://github.com/suppers-ai/solobase',
  discordUrl: 'https://discord.gg/jKqMcbrVzm',
};

export const mainMenu = [
  { name: 'Home', url: '/' },
  { name: 'Pricing', url: '/pricing/' },
  { name: 'Documentation', url: '/docs/' },
  { name: 'Discord', url: 'https://discord.gg/jKqMcbrVzm', external: true },
  { name: 'GitHub', url: 'https://github.com/suppers-ai/solobase', external: true },
  { name: 'Sign In', url: 'https://cloud.solobase.dev/blocks/dashboard/', external: true },
];

export const docsSidebar = [
  {
    title: 'Getting Started',
    items: [
      { name: 'Overview', path: '/docs/' },
      { name: 'Installation', path: '/docs/installation/' },
      { name: 'Configuration', path: '/docs/configuration/' },
      { name: 'Quick Start', path: '/docs/quick-start/' },
    ],
  },
  {
    title: 'Core Features',
    items: [
      { name: 'Dashboard', path: '/docs/dashboard/' },
      { name: 'Extensions', path: '/docs/extensions/' },
      { name: 'WASM Blocks', path: '/docs/wasm/' },
    ],
  },
  {
    title: 'API Reference',
    items: [
      { name: 'Authentication', path: '/docs/api/auth/' },
      { name: 'Database API', path: '/docs/api/database/' },
    ],
  },
  {
    title: 'Deployment',
    items: [
      { name: 'Docker', path: '/docs/deployment/docker/' },
      { name: 'Solobase Cloud', path: '/docs/cloud/' },
    ],
  },
];

export const footerResources = [
  { name: 'Installation', url: '/docs/installation/' },
  { name: 'Configuration', url: '/docs/configuration/' },
  { name: 'API Reference', url: '/docs/api/auth/' },
  { name: 'Deployment', url: '/docs/deployment/docker/' },
];
