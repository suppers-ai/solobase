import { defineConfig } from 'vite';
import preact from '@preact/preset-vite';
import { resolve } from 'path';
import { readFileSync } from 'fs';

// Serve public/ directory index files (e.g. /docs/ -> public/docs/index.html)
function servePublicIndex() {
  return {
    name: 'serve-public-index',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (req.url?.endsWith('/')) {
          const filePath = resolve(__dirname, 'public', req.url.slice(1), 'index.html');
          try {
            const html = readFileSync(filePath, 'utf-8');
            res.setHeader('Content-Type', 'text/html');
            res.end(html);
            return;
          } catch {}
        }
        next();
      });
    },
  };
}

export default defineConfig({
  plugins: [servePublicIndex(), preact()],
  build: {
    outDir: 'dist/site',
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        'use-cases': resolve(__dirname, 'use-cases/index.html'),
      },
    },
  },
});
