import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { execSync } from 'child_process';

const API_PORT = process.env.API_PORT || '8090';

// Get git commit hash at build time (fallback to 'demo' if not in git repo)
let gitHash = 'demo';
try {
	gitHash = execSync('git rev-parse --short HEAD').toString().trim();
} catch (e) {
	// Not in a git repository (e.g., Docker build), use default
	console.log('Not in git repository, using default version: demo');
}
const buildDate = new Date().toISOString().replace('T', ' ').split('.')[0];

export default defineConfig({
	plugins: [sveltekit()],
	define: {
		__APP_VERSION__: JSON.stringify(gitHash),
		__BUILD_DATE__: JSON.stringify(buildDate)
	},
	server: {
		proxy: {
			'/api': {
				target: `http://localhost:${API_PORT}`,
				changeOrigin: true,
				secure: false,
				// Configure cookie rewriting to ensure cookies work through the proxy
				configure: (proxy) => {
					proxy.on('proxyRes', (proxyRes) => {
						// Rewrite Set-Cookie headers to work with the dev server
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
