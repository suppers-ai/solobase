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
	plugins: [preact()],
	define: {
		__APP_VERSION__: JSON.stringify(gitHash),
		__BUILD_DATE__: JSON.stringify(buildDate)
	},
	build: {
		outDir: 'build',
		emptyOutDir: true,
		rollupOptions: {
			input: {
				login: resolve(__dirname, 'src/pages/login/index.html'),
				signup: resolve(__dirname, 'src/pages/signup/index.html'),
				logout: resolve(__dirname, 'src/pages/logout/index.html'),
				profile: resolve(__dirname, 'src/pages/profile/index.html'),
				'profile-products': resolve(__dirname, 'src/pages/profile-products/index.html'),
				products: resolve(__dirname, 'src/pages/products/index.html'),
				checkout: resolve(__dirname, 'src/pages/checkout/index.html'),
				success: resolve(__dirname, 'src/pages/success/index.html'),
				'oauth-callback': resolve(__dirname, 'src/pages/oauth-callback/index.html'),
			}
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
