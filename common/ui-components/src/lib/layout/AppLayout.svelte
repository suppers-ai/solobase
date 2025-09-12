<script lang="ts">
	import { Menu, X } from 'lucide-svelte';
	import { onMount } from 'svelte';
	import Sidebar from './Sidebar.svelte';
	
	export let currentUser: { email?: string; role?: string } | null = null;
	export let navigation: any[] = [];
	export let currentPath: string = '/';
	export let logoSrc: string = '/logo_long.png';
	export let logoCollapsedSrc: string = '/logo.png';
	export let projectName: string = 'Project';
	export let mobileTitle: string = projectName;
	export let onLogout: () => void = () => {};
	export let onOpenSettings: (() => void) | null = null;
	export let sidebarCollapsed: boolean = false;
	
	let mobileMenuOpen = false;
	let windowWidth = 0;
	
	onMount(() => {
		windowWidth = window.innerWidth;
		
		const handleResize = () => {
			windowWidth = window.innerWidth;
			// Close mobile menu on desktop resize
			if (windowWidth >= 768) {
				mobileMenuOpen = false;
			}
		};
		
		window.addEventListener('resize', handleResize);
		return () => window.removeEventListener('resize', handleResize);
	});
	
	function toggleMobileMenu() {
		mobileMenuOpen = !mobileMenuOpen;
	}
</script>

<div class="app-layout {mobileMenuOpen ? 'mobile-menu-open' : ''}">
	<!-- Mobile Header -->
	<header class="mobile-header">
		<button class="menu-toggle" on:click={toggleMobileMenu}>
			{#if mobileMenuOpen}
				<X size={24} />
			{:else}
				<Menu size={24} />
			{/if}
		</button>
		<div class="mobile-title">{mobileTitle}</div>
	</header>
	
	<!-- Sidebar with overlay for mobile -->
	<div class="sidebar-container {mobileMenuOpen ? 'active' : ''}">
		{#if windowWidth < 768 && mobileMenuOpen}
			<div class="sidebar-overlay" on:click={toggleMobileMenu}></div>
		{/if}
		<div class="sidebar-wrapper">
			<Sidebar 
				{currentUser}
				{navigation}
				{currentPath}
				{logoSrc}
				{logoCollapsedSrc}
				{projectName}
				{onLogout}
				{onOpenSettings}
				bind:collapsed={sidebarCollapsed}
			/>
		</div>
	</div>
	
	<main class="main-content">
		<div class="content-wrapper">
			<slot />
		</div>
	</main>
</div>

<style>
	@import '../css/layout.css';
</style>