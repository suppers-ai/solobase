import { defineConfig } from 'vite';
import preact from '@preact/preset-vite';
import { resolve } from 'path';

export default defineConfig({
  plugins: [preact()],
  build: {
    outDir: 'dist/site',
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        pricing: resolve(__dirname, 'pricing/index.html'),
        docs: resolve(__dirname, 'docs/index.html'),
      },
      output: {
        manualChunks: {
          'highlight': ['highlight.js'],
        },
      },
    },
  },
});
