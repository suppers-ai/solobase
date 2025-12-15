<script lang="ts">
	export let size: 'sm' | 'md' | 'lg' = 'md';
	export let color: 'primary' | 'white' | 'gray' = 'primary';
	export let centered: boolean = false;

	const sizes = {
		sm: { width: '16px', border: '2px' },
		md: { width: '32px', border: '3px' },
		lg: { width: '40px', border: '4px' }
	};

	const colors = {
		primary: { track: '#e5e7eb', spinner: '#189AB4' },
		white: { track: 'rgba(255, 255, 255, 0.3)', spinner: 'white' },
		gray: { track: '#e2e8f0', spinner: '#6b7280' }
	};

	$: sizeStyles = sizes[size];
	$: colorStyles = colors[color];
</script>

<div
	class="spinner"
	class:centered
	style="
		width: {sizeStyles.width};
		height: {sizeStyles.width};
		border-width: {sizeStyles.border};
		border-color: {colorStyles.track};
		border-top-color: {colorStyles.spinner};
	"
></div>

<style>
	.spinner {
		border-style: solid;
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	.spinner.centered {
		margin: 0 auto;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
