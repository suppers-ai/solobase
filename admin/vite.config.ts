import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

const API_PORT = process.env.API_PORT || '8090';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		proxy: {
			'/api': {
				target: `http://localhost:${API_PORT}`,
				changeOrigin: true,
				secure: false
			}
		}
	}
});
