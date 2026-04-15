export const siteConfig = {
  title: "Solobase - Modern Admin Dashboard",
  description: "Open source backend in a single binary",
  author: "Suppers Software Limited",
  logo: "/images/logo_long.png",
  demoUrl: "https://demo.solobase.dev",
  githubUrl: "https://github.com/suppers-ai/solobase",
  discordUrl: "https://discord.gg/jKqMcbrVzm",
};

export const mainMenu = [
  { name: "Home", url: "/" },
  { name: "Use Cases", url: "/use-cases/" },
  { name: "Docs", url: "/docs/" },
  { name: "Demo", url: "https://demo.solobase.dev", external: true },
];

export const socialLinks = [
  { name: "Discord", url: "https://discord.gg/jKqMcbrVzm", icon: "discord" },
  {
    name: "GitHub",
    url: "https://github.com/suppers-ai/solobase",
    icon: "github",
  },
];

export const docsSidebar = [
  {
    title: "Getting Started",
    items: [
      { name: "Overview", path: "/docs/" },
      { name: "Installation", path: "/docs/installation/" },
      { name: "Configuration", path: "/docs/configuration/" },
      { name: "Quick Start", path: "/docs/quick-start/" },
    ],
  },
  {
    title: "Core Features",
    items: [
      { name: "Dashboard", path: "/docs/dashboard/" },
      { name: "Extensions", path: "/docs/extensions/" },
      { name: "WASM Blocks", path: "/docs/wasm/" },
    ],
  },
  {
    title: "API Reference",
    items: [
      { name: "Authentication", path: "/docs/api/auth/" },
      { name: "Database API", path: "/docs/api/database/" },
    ],
  },
  {
    title: "Deployment",
    items: [
      { name: "Docker", path: "/docs/deployment/docker/" },
    ],
  },
];

export const footerResources = [
  { name: "Installation", url: "/docs/installation/" },
  { name: "Configuration", url: "/docs/configuration/" },
  { name: "API Reference", url: "/docs/api/auth/" },
  { name: "Deployment", url: "/docs/deployment/docker/" },
];
