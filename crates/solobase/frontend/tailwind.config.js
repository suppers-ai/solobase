import typography from '@tailwindcss/typography';

/** @type {import('tailwindcss').Config} */
export default {
	darkMode: 'class',
	content: [
		'../blocks/*/frontend/**/*.{html,ts,tsx}',
		'../../shared/ui/src/**/*.{ts,tsx}',
		'./static/**/*.html'
	],
	theme: {
		extend: {
			colors: {
				brand: {
					50: '#eef2ff',
					100: '#e0e7ff',
					200: '#c7d2fe',
					300: '#a5b4fc',
					400: '#818cf8',
					500: '#6366f1',
					600: '#4f46e5',
					700: '#4338ca',
					800: '#3730a3',
					900: '#312e81',
				}
			}
		}
	},
	plugins: [
		typography,
	]
};
