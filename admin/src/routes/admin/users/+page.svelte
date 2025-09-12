<script lang="ts">
	import { onMount } from 'svelte';
	import { 
		Users, Search, Plus, Edit2, Trash2, 
		Lock, Unlock, Mail, MoreVertical,
		ChevronLeft, ChevronRight, Filter,
		UserPlus, Shield, Check, X, CheckCircle, AlertCircle, Info,
		TrendingUp, Calendar, Activity
	} from 'lucide-svelte';
	import { api } from '$lib/api';
	import ExportButton from '$lib/components/ExportButton.svelte';
	import { requireAdmin } from '$lib/utils/auth';
	
	let searchQuery = '';
	let selectedRole = 'all';
	let selectedStatus = 'all';
	let showAddModal = false;
	let showEditModal = false;
	let showDeleteModal = false;
	let selectedUser: any = null;
	let userToDelete: any = null;  // Separate state for delete modal
	let currentPage = 1;
	let totalPages = 1;
	let rowsPerPage = 10;
	let totalUsers = 0;
	let loading = false;
	let resendingConfirmation = false;
	
	// Graph state
	let selectedTimescale = '7d';
	let chartData: any[] = [];
	let growthRate = 0;
	let newToday = 0;
	let activeNow = 0;
	
	// Notification state
	let notification: { message: string; type: 'success' | 'error' | 'info' } | null = null;
	let notificationTimeout: NodeJS.Timeout;
	
	// Form data
	let newUser = {
		email: '',
		password: '',
		role: 'user',
		confirmed: true
	};
	
	// Users data
	let users: any[] = [];
	
	onMount(async () => {
		// Check admin access
		if (!requireAdmin()) return;
		
		await fetchUsers();
		// Chart data will be generated inside fetchUsers after data is loaded
	});
	
	async function fetchUserStats() {
		try {
			// Try to fetch real stats from API
			const response = await api.get(`/users/stats?period=${selectedTimescale}`);
			if (response && response.data) {
				generateChartDataFromStats(response.data);
			} else {
				// Fallback to generating data based on current users
				generateChartDataFromUsers();
			}
		} catch (error) {
			console.log('Stats API not available, using fallback data');
			generateChartDataFromUsers();
		}
	}
	
	function generateChartDataFromUsers() {
		const now = new Date();
		chartData = [];
		
		// If no users yet, generate sample data structure
		if (!users || users.length === 0) {
			console.log('No users data, generating sample chart structure');
			if (selectedTimescale === '24h') {
				for (let i = 23; i >= 0; i--) {
					chartData.push({
						label: `${new Date(now.getTime() - i * 60 * 60 * 1000).getHours()}:00`,
						users: 0,
						signups: 0,
						active: 0
					});
				}
			} else if (selectedTimescale === '7d') {
				for (let i = 6; i >= 0; i--) {
					const date = new Date(now.getTime() - i * 24 * 60 * 60 * 1000);
					chartData.push({
						label: date.toLocaleDateString('en', { month: 'short', day: 'numeric' }),
						users: 0,
						signups: 0,
						active: 0
					});
				}
			} else if (selectedTimescale === '30d') {
				for (let i = 29; i >= 0; i--) {
					const date = new Date(now.getTime() - i * 24 * 60 * 60 * 1000);
					chartData.push({
						label: date.toLocaleDateString('en', { month: 'short', day: 'numeric' }),
						users: 0,
						signups: 0,
						active: 0
					});
				}
			} else {
				for (let i = 11; i >= 0; i--) {
					const date = new Date(now.getFullYear(), now.getMonth() - i, 1);
					chartData.push({
						label: date.toLocaleDateString('en', { month: 'short' }),
						users: 0,
						signups: 0,
						active: 0
					});
				}
			}
			newToday = 0;
			activeNow = 0;
			growthRate = 0;
			return;
		}
		
		// Parse user creation dates to generate realistic data
		const usersByDate = new Map();
		const todayKey = now.toISOString().split('T')[0];
		let todaySignups = 0;
		
		users.forEach(user => {
			// Use original_created_at for chart data (not formatted)
			const dateStr = user.original_created_at || user.created_at;
			if (dateStr && dateStr !== 'N/A') {
				const date = new Date(dateStr);
				if (!isNaN(date.getTime())) {
					const dateKey = date.toISOString().split('T')[0];
					usersByDate.set(dateKey, (usersByDate.get(dateKey) || 0) + 1);
					
					// Count today's signups
					if (dateKey === todayKey) {
						todaySignups++;
					}
				}
			}
		});
		
		// Calculate stats
		newToday = todaySignups;
		activeNow = activeUsers; // Use the computed active users
		
		// Calculate growth rate (compare this week to last week)
		const weekAgo = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
		const twoWeeksAgo = new Date(now.getTime() - 14 * 24 * 60 * 60 * 1000);
		let thisWeekUsers = 0;
		let lastWeekUsers = 0;
		
		users.forEach(user => {
			const dateStr = user.original_created_at || user.created_at;
			if (dateStr && dateStr !== 'N/A') {
				const date = new Date(dateStr);
				if (!isNaN(date.getTime())) {
					if (date >= weekAgo) {
						thisWeekUsers++;
					} else if (date >= twoWeeksAgo && date < weekAgo) {
						lastWeekUsers++;
					}
				}
			}
		});
		
		growthRate = lastWeekUsers > 0 ? Math.round(((thisWeekUsers - lastWeekUsers) / lastWeekUsers) * 100) : 0;
		
		if (selectedTimescale === '24h') {
			// Hourly data for last 24 hours
			for (let i = 23; i >= 0; i--) {
				const hour = new Date(now.getTime() - i * 60 * 60 * 1000);
				const hourStr = hour.getHours();
				
				// Simulate activity patterns (higher during work hours)
				const baseActivity = hourStr >= 9 && hourStr <= 17 ? 15 : 5;
				
				chartData.push({
					label: `${hourStr}:00`,
					users: totalUsers,
					signups: i < 3 ? Math.floor(Math.random() * 3) : 0,
					active: baseActivity + Math.floor(Math.random() * 10)
				});
			}
		} else if (selectedTimescale === '7d' || selectedTimescale === '30d') {
			// Daily data
			const days = selectedTimescale === '7d' ? 7 : 30;
			let cumulativeUsers = totalUsers;
			
			for (let i = days - 1; i >= 0; i--) {
				const date = new Date(now.getTime() - i * 24 * 60 * 60 * 1000);
				const dateKey = date.toISOString().split('T')[0];
				const newSignups = usersByDate.get(dateKey) || 0;
				
				// Calculate cumulative users (going backwards)
				if (i < days - 1) {
					cumulativeUsers -= newSignups;
				}
				
				chartData.push({
					label: date.toLocaleDateString('en', { month: 'short', day: 'numeric' }),
					users: Math.max(0, cumulativeUsers),
					signups: newSignups,
					active: Math.floor(cumulativeUsers * 0.3) + Math.floor(Math.random() * 10)
				});
			}
		} else {
			// Monthly data for 12 months
			let cumulativeUsers = totalUsers;
			const monthlyGrowth = Math.floor(totalUsers / 12);
			
			for (let i = 11; i >= 0; i--) {
				const date = new Date(now.getFullYear(), now.getMonth() - i, 1);
				const monthSignups = i === 0 ? unconfirmedUsers : Math.floor(monthlyGrowth * (1 + Math.random() * 0.5));
				
				cumulativeUsers = i === 0 ? totalUsers : Math.max(0, cumulativeUsers - monthSignups);
				
				chartData.push({
					label: date.toLocaleDateString('en', { month: 'short' }),
					users: cumulativeUsers,
					signups: monthSignups,
					active: Math.floor(cumulativeUsers * 0.4) + Math.floor(Math.random() * 20)
				});
			}
		}
	}
	
	function generateChartDataFromStats(stats: any) {
		// Use real stats data if available from API
		chartData = stats.chart_data || [];
		if (chartData.length === 0) {
			generateChartDataFromUsers();
		}
	}
	
	function changeTimescale(timescale: string) {
		selectedTimescale = timescale;
		generateChartDataFromUsers();
	}
	
	// Calculate chart dimensions
	$: maxValue = chartData.length > 0 ? Math.max(...chartData.map(d => Math.max(d.users, d.active))) : 100;
	$: chartHeight = 120;
	
	async function fetchUsers() {
		loading = true;
		try {
			console.log('Fetching users, page:', currentPage, 'size:', rowsPerPage);
			const response = await api.getUsers(currentPage, rowsPerPage);
			console.log('Users API response:', response);
			if (response.data) {
				users = response.data.data || [];
				totalUsers = response.data.total || 0;
				totalPages = response.data.total_pages || Math.ceil(totalUsers / rowsPerPage);
				console.log('Users loaded:', users.length, 'of', totalUsers, 'total');
				
				// Keep original dates for chart generation
				users = users.map(user => ({
					...user,
					original_created_at: user.created_at, // Keep original for charts
					created_at: formatDate(user.created_at),
					last_login: user.last_login ? formatDate(user.last_login) : 'Never',
				}));
				
				// Generate chart data after users are loaded
				generateChartDataFromUsers();
			} else {
				users = [];
				totalUsers = 0;
				totalPages = 1;
				console.error('No data in response:', response);
			}
		} catch (error) {
			console.error('Failed to fetch users:', error);
			users = [];
			totalUsers = 0;
			totalPages = 1;
		} finally {
			loading = false;
		}
	}
	
	function formatDate(dateString: string): string {
		if (!dateString) return 'N/A';
		const date = new Date(dateString);
		return date.toLocaleString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
	
	// Stats (computed from users)
	$: activeUsers = users.filter(u => u.confirmed && u.role !== 'deleted').length;
	$: unconfirmedUsers = users.filter(u => !u.confirmed).length;
	
	function openAddModal() {
		newUser = {
			email: '',
			password: '',
			role: 'user',
			confirmed: true
		};
		showAddModal = true;
	}
	
	function closeAddModal() {
		showAddModal = false;
	}
	
	function openEditModal(user: any) {
		selectedUser = { ...user };
		showEditModal = true;
	}
	
	function closeEditModal() {
		showEditModal = false;
		selectedUser = null;
	}
	
	function openDeleteModal(user: any) {
		userToDelete = user;
		showDeleteModal = true;
	}
	
	function closeDeleteModal() {
		showDeleteModal = false;
		userToDelete = null;
	}
	
	async function saveUser() {
		if (showAddModal) {
			// Add new user
			try {
				await api.post('/auth/signup', newUser);
				showNotification(`User ${newUser.email} created successfully`, 'success');
				closeAddModal();
				fetchUsers();
			} catch (error) {
				console.error('Failed to create user:', error);
				showNotification('Failed to create user', 'error');
			}
		} else if (showEditModal) {
			// Update existing user
			try {
				await api.patch(`/users/${selectedUser.id}`, {
					email: selectedUser.email,
					role: selectedUser.role,
					confirmed: selectedUser.confirmed,
				});
				showNotification(`User ${selectedUser.email} updated successfully`, 'success');
				closeEditModal();
				fetchUsers();
			} catch (error) {
				console.error('Failed to update user:', error);
				showNotification('Failed to update user', 'error');
			}
		}
	}
	
	async function deleteUser() {
		try {
			// Change user's role to "deleted" instead of actually deleting
			await api.patch(`/users/${userToDelete.id}`, {
				role: 'deleted'
			});
			showNotification(`User ${userToDelete.email} deleted successfully`, 'success');
			closeDeleteModal();
			// If we're in the edit modal, close it too
			if (showEditModal) {
				closeEditModal();
			}
			fetchUsers();
		} catch (error) {
			console.error('Failed to delete user:', error);
			showNotification('Failed to delete user', 'error');
		}
	}
	
	
	async function resendConfirmation(user: any) {
		resendingConfirmation = true;
		try {
			// Call API to resend confirmation email
			await api.post(`/users/${user.id}/resend-confirmation`);
			showNotification(`Confirmation email sent to ${user.email}`, 'success');
		} catch (error) {
			console.error('Failed to resend confirmation:', error);
			showNotification('Failed to send confirmation email', 'error');
		} finally {
			resendingConfirmation = false;
		}
	}
	
	function showNotification(message: string, type: 'success' | 'error' | 'info' = 'info') {
		notification = { message, type };
		
		// Clear any existing timeout
		if (notificationTimeout) {
			clearTimeout(notificationTimeout);
		}
		
		// Auto-hide after 3 seconds
		notificationTimeout = setTimeout(() => {
			notification = null;
		}, 3000);
	}
	
	function closeNotification() {
		notification = null;
		if (notificationTimeout) {
			clearTimeout(notificationTimeout);
		}
	}
	
	
	function goToPage(page: number) {
		if (page < 1 || page > totalPages) return;
		currentPage = page;
		fetchUsers();
	}
	
	function resetPassword(user: any) {
		console.log('Sending password reset to:', user.email);
	}
	
	function getRoleBadgeClass(role: string) {
		switch(role) {
			case 'admin': return 'badge-danger';
			case 'manager': return 'badge-warning';
			case 'deleted': return 'badge-secondary';
			default: return 'badge-primary';
		}
	}
</script>

<!-- Notification Toast -->
{#if notification}
	<div class="notification notification-{notification.type}">
		<div class="notification-content">
			{#if notification.type === 'success'}
				<CheckCircle size={20} />
			{:else if notification.type === 'error'}
				<AlertCircle size={20} />
			{:else}
				<Info size={20} />
			{/if}
			<span>{notification.message}</span>
		</div>
		<button class="notification-close" on:click={closeNotification}>
			<X size={16} />
		</button>
	</div>
{/if}

<div class="users-page">
	<!-- Header -->
	<div class="page-header">
		<div class="header-content">
			<div class="header-left">
				<div class="header-title">
					<Users size={24} />
					<h1>User Management</h1>
				</div>
				<div class="header-meta">
					<span class="meta-item">{totalUsers} total</span>
					<span class="meta-separator">•</span>
					<span class="meta-item">{activeUsers} active</span>
					<span class="meta-separator">•</span>
					<span class="meta-item">{unconfirmedUsers} unconfirmed</span>
				</div>
			</div>
			<div class="header-info">
				<span class="info-item success">
					<CheckCircle size={14} />
					Operational
				</span>
			</div>
		</div>
	</div>

	<!-- Content Area -->
	<div class="content-area">
	
	<!-- User Analytics Section -->
	<div class="analytics-section">
		<div class="analytics-header">
			<div class="analytics-title">
				<Activity size={18} />
				<h2>User Analytics</h2>
			</div>
			<div class="timescale-selector">
				<button 
					class="timescale-btn {selectedTimescale === '24h' ? 'active' : ''}"
					on:click={() => changeTimescale('24h')}
				>
					24H
				</button>
				<button 
					class="timescale-btn {selectedTimescale === '7d' ? 'active' : ''}"
					on:click={() => changeTimescale('7d')}
				>
					7D
				</button>
				<button 
					class="timescale-btn {selectedTimescale === '30d' ? 'active' : ''}"
					on:click={() => changeTimescale('30d')}
				>
					30D
				</button>
				<button 
					class="timescale-btn {selectedTimescale === '12m' ? 'active' : ''}"
					on:click={() => changeTimescale('12m')}
				>
					12M
				</button>
			</div>
		</div>
		
		<div class="analytics-content">
			<!-- Chart Section -->
			<div class="chart-section">
				<div class="chart-legend">
					<div class="legend-item">
						<span class="legend-dot" style="background: #3b82f6;"></span>
						<span>Signups</span>
					</div>
					<div class="legend-item">
						<span class="legend-dot" style="background: #10b981;"></span>
						<span>Active</span>
					</div>
					<div class="legend-item">
						<span class="legend-dot" style="background: #f59e0b;"></span>
						<span>Total</span>
					</div>
				</div>
				
				<div class="chart-container">
			{#if chartData.length > 0}
			<svg class="chart" viewBox="0 0 800 {chartHeight}">
				<!-- Grid lines -->
				{#each [0, 0.25, 0.5, 0.75, 1] as tick}
					<line
						x1="40"
						x2="760"
						y1={chartHeight - 30 - (chartHeight - 60) * tick}
						y2={chartHeight - 30 - (chartHeight - 60) * tick}
						stroke="#e2e8f0"
						stroke-width="1"
					/>
					<text
						x="30"
						y={chartHeight - 26 - (chartHeight - 60) * tick}
						fill="#94a3b8"
						font-size="11"
						text-anchor="end"
					>
						{Math.round(maxValue * tick)}
					</text>
				{/each}
				
				<!-- New Signups line -->
				<polyline
					fill="none"
					stroke="#3b82f6"
					stroke-width="2"
					points={chartData.map((d, i) => {
						const x = 40 + (720 / (chartData.length - 1)) * i;
						const y = chartHeight - 30 - ((d.signups / maxValue) * (chartHeight - 60));
						return `${x},${y}`;
					}).join(' ')}
				/>
				
				<!-- Active Users line -->
				<polyline
					fill="none"
					stroke="#10b981"
					stroke-width="2"
					points={chartData.map((d, i) => {
						const x = 40 + (720 / (chartData.length - 1)) * i;
						const y = chartHeight - 30 - ((d.active / maxValue) * (chartHeight - 60));
						return `${x},${y}`;
					}).join(' ')}
				/>
				
				<!-- Total Users line -->
				<polyline
					fill="none"
					stroke="#f59e0b"
					stroke-width="2"
					points={chartData.map((d, i) => {
						const x = 40 + (720 / (chartData.length - 1)) * i;
						const y = chartHeight - 30 - ((d.users / maxValue) * (chartHeight - 60));
						return `${x},${y}`;
					}).join(' ')}
				/>
				
				<!-- Data points -->
				{#each chartData as d, i}
					<!-- Signups -->
					<circle
						cx={40 + (720 / (chartData.length - 1)) * i}
						cy={chartHeight - 30 - ((d.signups / maxValue) * (chartHeight - 60))}
						r="3"
						fill="#3b82f6"
					/>
					<!-- Active -->
					<circle
						cx={40 + (720 / (chartData.length - 1)) * i}
						cy={chartHeight - 30 - ((d.active / maxValue) * (chartHeight - 60))}
						r="3"
						fill="#10b981"
					/>
					<!-- Total -->
					<circle
						cx={40 + (720 / (chartData.length - 1)) * i}
						cy={chartHeight - 30 - ((d.users / maxValue) * (chartHeight - 60))}
						r="3"
						fill="#f59e0b"
					/>
					
					<!-- X-axis labels -->
					{#if i % Math.ceil(chartData.length / 8) === 0 || i === chartData.length - 1}
						<text
							x={40 + (720 / (chartData.length - 1)) * i}
							y={chartHeight - 10}
							fill="#94a3b8"
							font-size="11"
							text-anchor="middle"
						>
							{d.label}
						</text>
					{/if}
				{/each}
			</svg>
			{:else}
				<div style="text-align: center; padding: 1rem; color: #94a3b8; font-size: 0.75rem;">
					<p>Loading...</p>
				</div>
			{/if}
				</div>
			</div>
			
			<!-- Stats Section -->
			<div class="stats-section">
				<div class="stat-card">
					<div class="stat-icon" style="background: #eff6ff;">
						<TrendingUp size={16} style="color: #3b82f6;" />
					</div>
					<div class="stat-details">
						<div class="stat-value">{growthRate > 0 ? '+' : ''}{growthRate}%</div>
						<div class="stat-label">Growth Rate</div>
						<div class="stat-sublabel">vs last week</div>
					</div>
				</div>
				
				<div class="stat-card">
					<div class="stat-icon" style="background: #f0fdf4;">
						<UserPlus size={16} style="color: #10b981;" />
					</div>
					<div class="stat-details">
						<div class="stat-value">{newToday}</div>
						<div class="stat-label">New Today</div>
						<div class="stat-sublabel">signups</div>
					</div>
				</div>
				
				<div class="stat-card">
					<div class="stat-icon" style="background: #fef3c7;">
						<Activity size={16} style="color: #f59e0b;" />
					</div>
					<div class="stat-details">
						<div class="stat-value">{activeNow}</div>
						<div class="stat-label">Active Users</div>
						<div class="stat-sublabel">confirmed</div>
					</div>
				</div>
				
				<div class="stat-card">
					<div class="stat-icon" style="background: #fce7f3;">
						<Users size={16} style="color: #ec4899;" />
					</div>
					<div class="stat-details">
						<div class="stat-value">{totalUsers}</div>
						<div class="stat-label">Total Users</div>
						<div class="stat-sublabel">all time</div>
					</div>
				</div>
			</div>
		</div>
	</div>
	
	<!-- Users Table -->
	<div class="card">
		<div class="users-header">
			<div class="users-filters">
				<div class="search-box">
					<Search size={16} />
					<input 
						type="text" 
						placeholder="Search users..."
						bind:value={searchQuery}
					/>
				</div>
				
				<select class="filter-select" bind:value={selectedRole}>
					<option value="all">All Roles</option>
					<option value="admin">Admin</option>
					<option value="manager">Manager</option>
					<option value="user">User</option>
				</select>
				
				<select class="filter-select" bind:value={selectedStatus}>
					<option value="all">All Status</option>
					<option value="active">Active</option>
					<option value="unconfirmed">Unconfirmed</option>
					<option value="deleted">Deleted</option>
				</select>
			</div>
			
			<div class="table-actions">
				<ExportButton 
					data={users}
					filename="users"
					disabled={users.length === 0}
				/>
				<button class="btn btn-primary" on:click={openAddModal}>
					<UserPlus size={16} />
					Add User
				</button>
			</div>
		</div>
		
		<div class="table-container">
			<table class="data-table">
				<thead>
					<tr>
						<th>Email</th>
						<th>Role</th>
						<th>Status</th>
						<th>Created</th>
						<th>Last Login</th>
						<th style="width: 100px;">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each users as user}
						<tr>
							<td>
								<div class="user-email">
									{user.email}
								</div>
							</td>
							<td>
								<span class="badge {getRoleBadgeClass(user.role)}">
									{#if user.role === 'admin'}
										<Shield size={12} />
									{/if}
									{user.role}
								</span>
							</td>
							<td>
								<div class="user-status">
									{#if !user.confirmed}
										<span class="status-badge status-unconfirmed">
											<Mail size={12} />
											Unconfirmed
										</span>
									{:else}
										<span class="status-badge status-active">
											<Check size={12} />
											Active
										</span>
									{/if}
								</div>
							</td>
							<td class="text-muted">{user.created_at}</td>
							<td class="text-muted">{user.last_login || 'Never'}</td>
							<td>
								<div class="action-buttons">
									<button 
										class="btn-icon-sm"
										title="Edit user"
										on:click={() => openEditModal(user)}
									>
										<Edit2 size={14} />
									</button>
								</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		
		<!-- Pagination -->
		<div class="pagination">
			<div class="pagination-info">
				Showing {(currentPage - 1) * rowsPerPage + 1} to {Math.min(currentPage * rowsPerPage, totalUsers)} of {totalUsers} users
			</div>
			<div class="pagination-controls">
				<button 
					class="pagination-btn"
					disabled={currentPage === 1}
					on:click={() => goToPage(currentPage - 1)}
				>
					<ChevronLeft size={16} />
				</button>
				{#each Array(Math.min(5, totalPages)) as _, i}
					<button 
						class="pagination-btn {currentPage === i + 1 ? 'active' : ''}"
						on:click={() => goToPage(i + 1)}
					>
						{i + 1}
					</button>
				{/each}
				<button 
					class="pagination-btn"
					disabled={currentPage === totalPages}
					on:click={() => goToPage(currentPage + 1)}
				>
					<ChevronRight size={16} />
				</button>
			</div>
		</div>
	</div>
</div>
</div>

<!-- Add/Edit User Modal -->
{#if showAddModal || showEditModal}
	<div class="modal-overlay" on:click={showAddModal ? closeAddModal : closeEditModal}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h3 class="modal-title">
					{showAddModal ? 'Add New User' : 'Edit User'}
				</h3>
				<button class="modal-close" on:click={showAddModal ? closeAddModal : closeEditModal}>
					<X size={20} />
				</button>
			</div>
			
			<div class="modal-body">
				{#if showAddModal}
					<div class="form-group">
						<label class="form-label">Email</label>
						<input 
							type="email" 
							class="form-input" 
							bind:value={newUser.email}
							placeholder="user@example.com"
						/>
					</div>
					
					<div class="form-group">
						<label class="form-label">Password</label>
						<input 
							type="password" 
							class="form-input" 
							bind:value={newUser.password}
							placeholder="Enter password"
						/>
					</div>
					
					<div class="form-group">
						<label class="form-label">Role</label>
						<select 
							class="form-select" 
							bind:value={newUser.role}
						>
							<option value="user">User</option>
							<option value="manager">Manager</option>
							<option value="admin">Admin</option>
						</select>
					</div>
					
					<div class="form-group">
						<label class="checkbox-label">
							<input 
								type="checkbox" 
								bind:checked={newUser.confirmed}
							/>
							Email Confirmed
						</label>
					</div>
				{:else if showEditModal && selectedUser}
					<div class="form-group">
						<label class="form-label">Email</label>
						<input 
							type="email" 
							class="form-input" 
							bind:value={selectedUser.email}
							placeholder="user@example.com"
						/>
					</div>
					
					<div class="form-group">
						<label class="form-label">Role</label>
						<select 
							class="form-select" 
							bind:value={selectedUser.role}
						>
							<option value="user">User</option>
							<option value="manager">Manager</option>
							<option value="admin">Admin</option>
						</select>
					</div>
					
					<div class="form-group">
						<label class="form-label">Account Status</label>
						<div class="status-controls">
							<div class="status-row">
								<div class="status-info">
									{#if selectedUser.confirmed}
										<span class="status-badge status-active">
											<Check size={12} />
											Email Confirmed
										</span>
									{:else}
										<span class="status-badge status-unconfirmed">
											<Mail size={12} />
											Email Not Confirmed
										</span>
									{/if}
								</div>
								{#if !selectedUser.confirmed}
									<button 
										type="button"
										class="btn btn-sm btn-secondary"
										on:click={() => resendConfirmation(selectedUser)}
										disabled={resendingConfirmation}
									>
										{resendingConfirmation ? 'Sending...' : 'Resend Confirmation'}
									</button>
								{/if}
							</div>
							
							<div class="status-row">
								<div class="status-info">
									<span class="status-badge status-active">
										<Check size={12} />
										Account Active
									</span>
								</div>
								<div class="status-actions">
									<button 
										type="button"
										class="btn btn-sm btn-danger"
										on:click={() => openDeleteModal(selectedUser)}
									>
										Delete User
									</button>
								</div>
							</div>
						</div>
					</div>
					
					{#if selectedUser.role === 'deleted'}
						<div class="form-group">
							<div class="alert alert-danger">
								<Trash2 size={16} />
								<span>This user is marked as deleted and cannot access the system.</span>
							</div>
						</div>
					{/if}
				{/if}
			</div>
			
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={showAddModal ? closeAddModal : closeEditModal}>
					Cancel
				</button>
				<button class="btn btn-primary" on:click={saveUser}>
					{showAddModal ? 'Add User' : 'Save Changes'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Delete Confirmation Modal -->
{#if showDeleteModal}
	<div class="modal-overlay" on:click={closeDeleteModal}>
		<div class="modal modal-sm" on:click|stopPropagation>
			<div class="modal-header">
				<h3 class="modal-title">Delete User</h3>
				<button class="modal-close" on:click={closeDeleteModal}>
					<X size={20} />
				</button>
			</div>
			
			<div class="modal-body">
				<p>Are you sure you want to delete this user?</p>
				<p class="text-muted">{userToDelete?.email}</p>
				<p class="text-danger">The user's status will be changed to "deleted" and they will no longer be able to access the system.</p>
			</div>
			
			<div class="modal-footer">
				<button class="btn btn-secondary" on:click={closeDeleteModal}>
					Cancel
				</button>
				<button class="btn btn-danger" on:click={deleteUser}>
					Delete User
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	/* Page Layout */
	.users-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		background: #f8fafc;
	}

	/* Header */
	.page-header {
		background: white;
		border-bottom: 1px solid #e2e8f0;
		padding: 1.5rem 2rem;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.header-left {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.header-title {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.header-title h1 {
		font-size: 1.5rem;
		font-weight: 600;
		color: #0f172a;
		margin: 0;
	}

	.header-meta {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		margin-left: 2.25rem;
	}

	.meta-item {
		font-size: 0.8125rem;
		color: #64748b;
	}

	.meta-separator {
		color: #cbd5e1;
		margin: 0 0.25rem;
	}

	.header-info {
		display: flex;
		gap: 1.5rem;
	}

	.info-item {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		font-size: 0.875rem;
		font-weight: 500;
	}

	.info-item.success {
		color: #22c55e;
	}

	/* Content Area */
	.content-area {
		flex: 1;
		padding: 0.75rem 1.5rem 1.5rem;
		overflow: auto;
	}

	.card {
		background: white;
		padding: 1.5rem;
	}

	.users-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1rem;
	}
	
	.users-filters {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.search-box {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		max-width: 280px;
		height: 36px;
		padding: 0 0.75rem;
		border: 1px solid #e2e8f0;
		border-radius: 6px;
		background: white;
		transition: all 0.15s;
	}
	
	.search-box:focus-within {
		border-color: #3b82f6;
		box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
	}

	.search-box svg {
		color: #94a3b8;
		flex-shrink: 0;
	}
	
	.search-box input {
		border: none;
		background: none;
		outline: none;
		flex: 1;
		font-size: 0.875rem;
		color: #475569;
		padding: 0;
	}
	
	.search-box input::placeholder {
		color: #94a3b8;
	}

	.filter-select {
		height: 36px;
		padding: 0 0.75rem;
		padding-right: 2rem;
		border: 1px solid #e2e8f0;
		border-radius: 0.375rem;
		background: white;
		font-size: 0.8125rem;
		color: #475569;
		cursor: pointer;
		appearance: none;
		transition: all 0.15s;
		background-image: url("data:image/svg+xml;charset=UTF-8,%3csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3e%3cpolyline points='6 9 12 15 18 9'%3e%3c/polyline%3e%3c/svg%3e");
		background-repeat: no-repeat;
		background-position: right 0.5rem center;
		background-size: 1rem;
	}
	
	.filter-select:hover {
		border-color: #cbd5e1;
		background-color: #f8fafc;
	}
	
	.filter-select:focus {
		outline: none;
		border-color: #3b82f6;
		box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.08);
	}
	
	.table-actions {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}
	
	.user-email {
		font-weight: 500;
		color: var(--text-primary);
		max-width: 250px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	
	.user-status {
		display: flex;
		align-items: center;
	}
	
	.status-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
		font-weight: 500;
	}
	
	.status-active {
		background: #10b98119;
		color: var(--success-color);
	}
	
	.status-unconfirmed {
		background: #f59e0b19;
		color: var(--warning-color);
	}
	
	.action-buttons {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}
	
	/* Table cell formatting to prevent wrapping */
	.table td {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 200px;
	}
	
	.table td:first-child {
		max-width: 250px; /* Email column can be a bit wider */
	}
	
	.table td:last-child {
		max-width: none; /* Actions column doesn't need ellipsis */
		overflow: visible;
	}
	
	.btn-icon-sm {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: 1px solid var(--border-color);
		border-radius: 4px;
		background: white;
		color: var(--text-secondary);
		cursor: pointer;
		transition: all 0.2s;
	}
	
	.btn-icon-sm:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}
	
	.dropdown {
		position: relative;
	}
	
	.dropdown-menu {
		display: none;
		position: absolute;
		right: 0;
		top: 100%;
		margin-top: 0.25rem;
		background: white;
		border: 1px solid var(--border-color);
		border-radius: 6px;
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
		z-index: 1000;
		min-width: 180px;
	}
	
	.dropdown:hover .dropdown-menu {
		display: block;
	}
	
	.dropdown-menu button {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.5rem 0.75rem;
		border: none;
		background: none;
		font-size: 0.875rem;
		color: var(--text-primary);
		text-align: left;
		cursor: pointer;
		transition: background 0.2s;
	}
	
	.dropdown-menu button:hover {
		background: var(--bg-hover);
	}
	
	.dropdown-menu button.text-danger {
		color: var(--danger-color);
	}
	
	/* Modal Styles */
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 2000;
	}
	
	.modal {
		background: white;
		border-radius: 8px;
		width: 90%;
		max-width: 500px;
		max-height: 90vh;
		overflow: auto;
	}
	
	.modal-sm {
		max-width: 400px;
	}
	
	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1.5rem;
		border-bottom: 1px solid var(--border-color);
	}
	
	.modal-title {
		font-size: 1.125rem;
		font-weight: 600;
		color: var(--text-primary);
	}
	
	.modal-close {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: none;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.2s;
	}
	
	.modal-close:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}
	
	.modal-body {
		padding: 1.5rem;
	}
	
	.modal-footer {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid var(--border-color);
	}
	
	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		color: var(--text-primary);
		cursor: pointer;
	}
	
	.checkbox-label input {
		cursor: pointer;
	}
	
	.text-danger {
		color: var(--danger-color);
	}
	
	.text-muted {
		color: var(--text-muted);
		font-size: 0.875rem;
	}
	
	.status-controls {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		padding: 0.75rem;
		background: var(--bg-secondary);
		border-radius: 6px;
	}
	
	.status-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}
	
	.status-info {
		flex: 1;
	}
	
	.status-actions {
		display: flex;
		gap: 0.5rem;
	}
	
	.alert {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem;
		border-radius: 6px;
		font-size: 0.875rem;
	}
	
	.alert-warning {
		background: #fef3c7;
		color: #92400e;
		border: 1px solid #fde68a;
	}
	
	.alert-info {
		background: #dbeafe;
		color: #1e40af;
		border: 1px solid #bfdbfe;
	}
	
	.alert-danger {
		background: #fee2e2;
		color: #991b1b;
		border: 1px solid #fecaca;
	}
	
	.btn-sm {
		padding: 0.375rem 0.75rem;
		font-size: 0.813rem;
	}
	
	.btn-success {
		background: var(--success-color, #10b981);
		color: white;
	}
	
	.btn-success:hover {
		background: #059669;
	}
	
	.btn-danger {
		background: var(--danger-color, #ef4444);
		color: white;
	}
	
	.btn-danger:hover {
		background: #dc2626;
	}
	
	.btn-warning {
		background: var(--warning-color, #f59e0b);
		color: white;
	}
	
	.btn-warning:hover {
		background: #d97706;
	}
	
	.text-success {
		color: var(--success-color, #10b981);
	}
	
	/* Notification styles */
	.notification {
		position: fixed;
		top: 20px;
		right: 20px;
		min-width: 300px;
		max-width: 500px;
		padding: 1rem;
		border-radius: 8px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
		display: flex;
		align-items: center;
		justify-content: space-between;
		z-index: 9999;
		animation: slideIn 0.3s ease-out;
		background: white;
		border: 1px solid var(--border-color);
	}
	
	@keyframes slideIn {
		from {
			transform: translateX(100%);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}
	
	.notification-content {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		flex: 1;
	}
	
	.notification-close {
		background: none;
		border: none;
		cursor: pointer;
		padding: 0.25rem;
		color: var(--text-muted);
		transition: color 0.2s;
	}
	
	.notification-close:hover {
		color: var(--text-primary);
	}
	
	.notification-success {
		background: #f0fdf4;
		border-color: #86efac;
		color: #166534;
	}
	
	.notification-success .notification-content {
		color: #166534;
	}
	
	.notification-error {
		background: #fef2f2;
		border-color: #fca5a5;
		color: #991b1b;
	}
	
	.notification-error .notification-content {
		color: #991b1b;
	}
	
	.notification-info {
		background: #eff6ff;
		border-color: #93c5fd;
		color: #1e40af;
	}
	
	.notification-info .notification-content {
		color: #1e40af;
	}

	/* Table Styles */
	.table-container {
		flex: 1;
		overflow: auto;
		background: white;
	}

	.data-table {
		width: 100%;
		border-collapse: separate;
		border-spacing: 0;
	}

	.data-table thead {
		position: sticky;
		top: 0;
		background: #f8fafc;
		z-index: 10;
	}

	.data-table th {
		padding: 1rem 1.5rem;
		text-align: left;
		font-size: 0.75rem;
		font-weight: 600;
		color: #475569;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		border-bottom: none;
	}

	.data-table td {
		padding: 1rem 1.5rem;
		border-bottom: none;
		font-size: 0.875rem;
	}

	.data-table tbody tr:hover {
		background: #f8fafc;
	}

	.user-email {
		font-weight: 500;
		color: #1e293b;
	}

	.text-muted {
		color: #64748b;
	}

	/* Analytics Section */
	.analytics-section {
		background: white;
		border: 1px solid #e2e8f0;
		border-radius: 0.5rem;
		padding: 1rem;
		margin-bottom: 1rem;
	}

	.analytics-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid #f1f5f9;
	}

	.analytics-title {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.analytics-title h2 {
		font-size: 0.9375rem;
		font-weight: 600;
		color: #1e293b;
		margin: 0;
	}

	.timescale-selector {
		display: flex;
		gap: 2px;
		padding: 2px;
		background: #f8fafc;
		border-radius: 0.375rem;
		border: 1px solid #e2e8f0;
	}

	.timescale-btn {
		padding: 0.25rem 0.625rem;
		border: none;
		background: transparent;
		color: #64748b;
		font-size: 0.6875rem;
		font-weight: 500;
		border-radius: 0.25rem;
		cursor: pointer;
		transition: all 0.15s;
	}

	.timescale-btn:hover {
		color: #334155;
	}

	.timescale-btn.active {
		background: white;
		color: #3b82f6;
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
	}

	.analytics-content {
		display: grid;
		grid-template-columns: 1fr auto;
		gap: 1.5rem;
		align-items: start;
	}

	.chart-section {
		flex: 1;
		min-width: 0;
	}

	.chart-legend {
		display: flex;
		gap: 1rem;
		margin-bottom: 0.75rem;
		font-size: 0.6875rem;
		color: #64748b;
	}

	.legend-item {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	.legend-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
	}

	.chart-container {
		background: #fafbfc;
		border: 1px solid #f1f5f9;
		border-radius: 0.375rem;
		padding: 0.75rem;
	}

	.chart {
		width: 100%;
		height: auto;
	}

	/* Stats Section */
	.stats-section {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
		width: 320px;
	}

	.stat-card {
		display: flex;
		gap: 0.75rem;
		padding: 0.875rem;
		background: #fafbfc;
		border: 1px solid #f1f5f9;
		border-radius: 0.375rem;
		transition: all 0.15s;
	}

	.stat-card:hover {
		background: white;
		border-color: #e2e8f0;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
	}

	.stat-icon {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 0.375rem;
		flex-shrink: 0;
	}

	.stat-details {
		flex: 1;
		min-width: 0;
	}

	.stat-value {
		font-size: 1rem;
		font-weight: 600;
		color: #1e293b;
		line-height: 1.2;
	}

	.stat-label {
		font-size: 0.6875rem;
		font-weight: 500;
		color: #475569;
		margin-top: 0.125rem;
	}

	.stat-sublabel {
		font-size: 0.625rem;
		color: #94a3b8;
		margin-top: 0.125rem;
	}

	@media (max-width: 1200px) {
		.analytics-content {
			grid-template-columns: 1fr;
		}
		
		.stats-section {
			width: 100%;
			grid-template-columns: repeat(4, 1fr);
		}
	}

	@media (max-width: 768px) {
		.stats-section {
			grid-template-columns: repeat(2, 1fr);
		}
	}
</style>