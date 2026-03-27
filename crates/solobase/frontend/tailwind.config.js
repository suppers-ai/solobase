import typography from '@tailwindcss/typography';

/** @type {import('tailwindcss').Config} */
export default {
	darkMode: 'class',
	content: [
		'../blocks/*/frontend/**/*.{html,ts,tsx}',
		'../../shared/ui/src/**/*.{ts,tsx}',
		'./static/**/*.html',
		'../../../packages/cloud-dashboard/**/*.{html,ts,tsx}',
	],
	theme: {
		extend: {
			colors: {
				brand: {
					50: '#fff4ed',
					100: '#ffe6d5',
					200: '#ffc9a8',
					300: '#ffa470',
					400: '#fe6627',
					500: '#fc4c03',
					600: '#b72a07',
					700: '#9a1f0a',
					800: '#7c1b0e',
					900: '#661a10',
				}
			}
		}
	},
	plugins: [
		typography,
	]
};
