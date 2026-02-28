import forms from '@tailwindcss/forms';
import typography from '@tailwindcss/typography';

/** @type {import('tailwindcss').Config} */
export default {
	darkMode: 'class',
	content: [
		'./src/**/*.{html,ts,tsx}',
		'../../shared/ui/src/**/*.{ts,tsx}'
	],
	theme: {
		extend: {}
	},
	plugins: [
		forms,
		typography,
	]
};
