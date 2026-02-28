import typography from '@tailwindcss/typography';

/** @type {import('tailwindcss').Config} */
export default {
	darkMode: 'class',
	content: [
		'../blocks/*/frontend/**/*.{html,ts,tsx}',
		'../../shared/ui/src/**/*.{ts,tsx}'
	],
	theme: {
		extend: {}
	},
	plugins: [
		typography,
	]
};
