<script lang="ts">
	import { 
		Home, Users, Database, HardDrive, 
		FileText, Puzzle, Settings as SettingsIcon, ChevronRight,
		ChevronLeft, LogOut, User, ChevronDown,
		ChevronUp, Menu, X
	} from 'lucide-svelte';
	import { onMount, onDestroy } from 'svelte';
	
	export let currentUser: { email?: string; role?: string } | null = null;
	export let collapsed: boolean = false;
	export let navigation: any[] = [];
	export let currentPath: string = '/';
	export let logoSrc: string = '/logo_long.png';
	export let logoCollapsedSrc: string = '/logo.png';
	export let projectName: string = 'Project';
	export let onLogout: () => void = () => {};
	export let onOpenSettings: () => void | null = null;
	export let profileUrl: string = '/profile';
	export let adminUrl: string = '/admin';
	
	let expandedItems: { [key: string]: boolean } = {};
	let showProfileMenu = false;
	
	function toggleExpanded(title: string) {
		expandedItems[title] = !expandedItems[title];
	}
	
	function toggleSidebar() {
		collapsed = !collapsed;
	}
	
	function toggleProfileMenu() {
		// Remove setTimeout to make it instant
		showProfileMenu = !showProfileMenu;
	}
	
	function handleClickOutside(event: MouseEvent) {
		const target = event.target as HTMLElement;
		if (!target.closest('.sidebar-user') && !target.closest('.profile-menu')) {
			showProfileMenu = false;
		}
	}
	
	function handleLogout() {
		onLogout();
	}
	
	// Automatically expand menu items when on their pages
	$: {
		navigation.forEach(item => {
			if (item.expandable && item.children) {
				const isActive = item.children.some((child: any) => 
					currentPath.startsWith(child.href)
				);
				if (isActive) {
					expandedItems[item.title] = true;
				}
			}
		});
	}
	
	onMount(() => {
		document.addEventListener('click', handleClickOutside);
	});
	
	onDestroy(() => {
		document.removeEventListener('click', handleClickOutside);
	});
</script>

<aside class="sidebar {collapsed ? 'collapsed' : ''}">
	<!-- Logo -->
	<div class="sidebar-header">
		<a href="/" class="sidebar-logo">
			{#if collapsed}
				<img src={logoCollapsedSrc} alt={projectName} width="36" height="36" />
			{:else}
				<img src={logoSrc} alt={projectName} height="50" style="height: 50px; width: auto;" />
			{/if}
		</a>
	</div>
	
	<!-- Navigation -->
	<nav class="sidebar-nav">
		{#each navigation as item}
			<div class="nav-item {expandedItems[item.title] ? 'expanded' : ''}">
				{#if item.expandable}
					<button 
						class="nav-link"
						on:click={() => toggleExpanded(item.title)}
					>
						<svelte:component this={item.icon} class="nav-icon" />
						{#if !collapsed}
							<span class="nav-text">{item.title}</span>
							<ChevronRight class="nav-expand-icon" />
						{/if}
					</button>
					{#if expandedItems[item.title]}
						<div class="nav-submenu">
							{#each item.children as child}
								<a 
									href={child.href}
									class="nav-link nav-subitem {currentPath === child.href ? 'active' : ''}"
								>
									{#if child.icon}
										<svelte:component this={child.icon} size={10} class="nav-subitem-icon" />
									{/if}
									{#if !collapsed}
										<span class="nav-text">{child.title}</span>
									{:else}
										<span class="nav-tooltip">{child.title}</span>
									{/if}
									{#if child.badge}
										<span class="nav-badge" style="background: var(--{child.badgeColor}-color)">
											{child.badge}
										</span>
									{/if}
								</a>
							{/each}
						</div>
					{/if}
				{:else}
					<a 
						href={item.href}
						class="nav-link {currentPath === item.href ? 'active' : ''}"
					>
						<svelte:component this={item.icon} class="nav-icon" />
						{#if !collapsed}
							<span class="nav-text">{item.title}</span>
						{:else}
							<span class="nav-tooltip">{item.title}</span>
						{/if}
					</a>
				{/if}
			</div>
		{/each}
		
		<!-- Collapse Toggle Button -->
		<button class="sidebar-toggle" on:click={toggleSidebar} title="{collapsed ? 'Expand' : 'Collapse'} sidebar">
			{#if collapsed}
				<ChevronRight size={16} />
			{:else}
				<ChevronLeft size={16} />
			{/if}
		</button>
	</nav>
	
	<!-- User Section -->
	{#if currentUser && currentUser.email}
		<div class="sidebar-user-container">
			<button class="sidebar-user" on:click={toggleProfileMenu}>
				<div class="user-avatar">
					{currentUser.email.substring(0, 1).toUpperCase()}
				</div>
				{#if !collapsed}
					<div class="user-info">
						<div class="user-email">{currentUser.email}</div>
					</div>
					{#if showProfileMenu}
						<ChevronUp size={16} style="color: var(--text-muted)" />
					{:else}
						<ChevronDown size={16} style="color: var(--text-muted)" />
					{/if}
				{/if}
			</button>
			
			<!-- Profile Menu Popup -->
			{#if showProfileMenu}
				<div class="profile-menu {collapsed ? 'profile-menu-collapsed' : ''}">
					<div class="profile-menu-header">
						<div class="profile-menu-avatar">
							{currentUser.email.substring(0, 1).toUpperCase()}
						</div>
						<div class="profile-menu-info">
							<div class="profile-menu-email">{currentUser.email}</div>
							<div class="profile-menu-role">{currentUser.role || 'User'}</div>
						</div>
					</div>
					<div class="profile-menu-divider"></div>
					
					<!-- Profile Link - opens Solobase profile page -->
					<a href={profileUrl} class="profile-menu-item" on:click={() => showProfileMenu = false}>
						<User size={16} />
						<span>My Profile</span>
					</a>
					
					{#if onOpenSettings}
						<button class="profile-menu-item" on:click={() => { showProfileMenu = false; onOpenSettings(); }}>
							<SettingsIcon size={16} />
							<span>Settings</span>
						</button>
					{/if}
					
					<!-- Admin Panel Link - only for admin users -->
					{#if currentUser.role === 'admin'}
						<div class="profile-menu-divider"></div>
						<a href={adminUrl} class="profile-menu-item" on:click={() => showProfileMenu = false}>
							<SettingsIcon size={16} />
							<span>Admin Panel</span>
						</a>
					{/if}
					
					<div class="profile-menu-divider"></div>
					<button class="profile-menu-item profile-menu-item-danger" on:click={handleLogout}>
						<LogOut size={16} />
						<span>Sign Out</span>
					</button>
				</div>
			{/if}
		</div>
	{/if}
</aside>

<style>
	/* Import the common sidebar styles */
	@import '../css/sidebar.css';
</style>