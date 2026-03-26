import { defineConfig } from 'vite';
import preact from '@preact/preset-vite';
import { execSync } from 'child_process';
import { resolve } from 'path';

const API_PORT = process.env.API_PORT || '8090';

// Get git commit hash at build time (fallback to 'demo' if not in git repo)
let gitHash = 'demo';
try {
	gitHash = execSync('git rev-parse --short HEAD').toString().trim();
} catch (e) {
	console.log('Not in git repository, using default version: demo');
}
const buildDate = new Date().toISOString().replace('T', ' ').split('.')[0];

export default defineConfig({
	root: resolve(__dirname, '..'),  // repo root
	publicDir: resolve(__dirname, 'static'),  // frontend/static/ → copied to build root
	plugins: [preact()],
	css: {
		postcss: __dirname,  // frontend/ — finds postcss.config.js + tailwind.config.js
	},
	define: {
		__APP_VERSION__: JSON.stringify(gitHash),
		__BUILD_DATE__: JSON.stringify(buildDate)
	},
	build: {
		outDir: resolve(__dirname, '..', 'data', 'storage', 'site'),  // data/storage/site/ (served by wafer-run/web via storage)
		emptyOutDir: true,
		sourcemap: true,
		rollupOptions: {
			input: {
				iam: resolve(__dirname, '../blocks/admin/frontend/iam/index.html'),
				logs: resolve(__dirname, '../blocks/logs/frontend/index.html'),
				wafer: resolve(__dirname, '../blocks/admin/frontend/index.html'),
				products: resolve(__dirname, '../blocks/products/frontend/index.html'),
				'products-user': resolve(__dirname, '../blocks/products/frontend/user/index.html'),
				deployments: resolve(__dirname, '../blocks/deployments/frontend/index.html'),
				dashboard: resolve(__dirname, '../blocks/dashboard/frontend/index.html'),
			}
		}
	},
	resolve: {
		alias: {
			'@app': resolve(__dirname, 'src'),
		}
	},
	server: {
		proxy: {
			'/api': {
				target: `http://localhost:${API_PORT}`,
				changeOrigin: true,
				secure: false,
				configure: (proxy) => {
					proxy.on('proxyRes', (proxyRes) => {
						const cookies = proxyRes.headers['set-cookie'];
						if (cookies) {
							proxyRes.headers['set-cookie'] = cookies.map((cookie: string) =>
								cookie.replace(/;\s*SameSite=Strict/gi, '; SameSite=Lax')
							);
						}
					});
				}
			}
		}
	}
});