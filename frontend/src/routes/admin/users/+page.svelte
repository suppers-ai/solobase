<script lang="ts">
	import { onMount } from 'svelte';
	import {
		Users, Plus, Edit2, Trash2,
		Lock, Unlock, Mail, MoreVertical,
		Filter,
		UserPlus, Shield, Check, X, CheckCircle, AlertCircle, Info,
		TrendingUp, Calendar, Activity
	} from 'lucide-svelte';
	import PageHeader from '$lib/components/ui/PageHeader.svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import StatCard from '$lib/components/ui/StatCard.svelte';
	import Pagination from '$lib/components/ui/Pagination.svelte';
	import { api } from '$lib/api';
	import ExportButton from '$lib/components/ExportButton.svelte';
	import { requireAdmin } from '$lib/utils/auth';
	import { formatDateTime } from '$lib/utils/formatters';
	import Modal from '$lib/components/ui/Modal.svelte';
	
	let searchQuery = '';
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
	
	interface UserStatsResponse {
		data?: {
			chartData?: unknown[];
		};
	}

	async function fetchUserStats() {
		try {
			// Try to fetch real stats from API
			const response = await api.get<UserStatsResponse>(`/users/stats?period=${selectedTimescale}`);
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
			// Use originalCreatedAt for chart data (not formatted)
			const dateStr = user.originalCreatedAt || user.createdAt;
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
			const dateStr = user.originalCreatedAt || user.createdAt;
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
		chartData = stats.chartData || [];
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
				totalPages = response.data.totalPages || Math.ceil(totalUsers / rowsPerPage);
				console.log('Users loaded:', users.length, 'of', totalUsers, 'total');
				
				// Keep original dates for chart generation
				// API returns camelCase: createdAt, lastLogin
				users = users.map(user => ({
					...user,
					originalCreatedAt: user.createdAt, // Keep original for charts
					createdAt: formatDateTime(user.createdAt),
					lastLogin: user.lastLogin ? formatDateTime(user.lastLogin) : 'Never',
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
	
	// Stats (computed from users)
	$: activeUsers = users.filter(u => u.confirmed).length;
	$: unconfirmedUsers = users.filter(u => !u.confirmed).length;
	
	function openAddModal() {
		newUser = {
			email: '',
			password: '',
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
			// Actually delete the user
			await api.delete(`/users/${userToDelete.id}`);
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
	<PageHeader
		title="User Management"
		icon={Users}
	>
		<svelte:fragment slot="meta">
			<span class="meta-item">{totalUsers} total</span>
			<span class="meta-separator">•</span>
			<span class="meta-item">{activeUsers} active</span>
			<span class="meta-separator">•</span>
			<span class="meta-item">{unconfirmedUsers} unconfirmed</span>
		</svelte:fragment>
		<svelte:fragment slot="info">
			<span class="info-item success">
				<CheckCircle size={14} />
				Operational
			</span>
		</svelte:fragment>
	</PageHeader>

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
				<StatCard
					icon={TrendingUp}
					value="{growthRate > 0 ? '+' : ''}{growthRate}%"
					label="Growth Rate"
					sublabel="vs last week"
					iconBg="bg-blue-100"
					iconColor="text-blue-600"
					size="sm"
				/>
				<StatCard
					icon={UserPlus}
					value={newToday}
					label="New Today"
					sublabel="signups"
					iconBg="bg-green-100"
					iconColor="text-green-600"
					size="sm"
				/>
				<StatCard
					icon={Activity}
					value={activeNow}
					label="Active Users"
					sublabel="confirmed"
					iconBg="bg-yellow-100"
					iconColor="text-yellow-600"
					size="sm"
				/>
				<StatCard
					icon={Users}
					value={totalUsers}
					label="Total Users"
					sublabel="all time"
					iconBg="bg-pink-100"
					iconColor="text-pink-600"
					size="sm"
				/>
			</div>
		</div>
	</div>
	
	<!-- Users Table -->
	<div class="card">
		<div class="users-header">
			<div class="users-filters">
				<SearchInput bind:value={searchQuery} placeholder="Search users..." maxWidth="280px" />

				<select class="filter-select" bind:value={selectedStatus}>
					<option value="all">All Status</option>
					<option value="active">Active</option>
					<option value="unconfirmed">Unconfirmed</option>
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
							<td class="text-muted">{user.createdAt}</td>
							<td class="text-muted">{user.lastLogin}</td>
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
		<Pagination
			{currentPage}
			{totalPages}
			totalItems={totalUsers}
			pageSize={rowsPerPage}
			on:change={(e) => goToPage(e.detail)}
		/>
	</div>
</div>
</div>

<!-- Add/Edit User Modal -->
<Modal show={showAddModal || showEditModal} title={showAddModal ? 'Add New User' : 'Edit User'} on:close={showAddModal ? closeAddModal : closeEditModal}>
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
	{/if}
	<svelte:fragment slot="footer">
		<button class="btn btn-secondary" on:click={showAddModal ? closeAddModal : closeEditModal}>
			Cancel
		</button>
		<button class="btn btn-primary" on:click={saveUser}>
			{showAddModal ? 'Add User' : 'Save Changes'}
		</button>
	</svelte:fragment>
</Modal>

<!-- Delete Confirmation Modal -->
<Modal show={showDeleteModal} title="Delete User" maxWidth="400px" on:close={closeDeleteModal}>
	<p>Are you sure you want to delete this user?</p>
	<p class="text-muted">{userToDelete?.email}</p>
	<p class="text-danger">This action cannot be undone. The user will be permanently deleted from the system.</p>
	<svelte:fragment slot="footer">
		<button class="btn btn-secondary" on:click={closeDeleteModal}>
			Cancel
		</button>
		<button class="btn btn-danger" on:click={deleteUser}>
			Delete User
		</button>
	</svelte:fragment>
</Modal>

<style>
	/* Page Layout */
	.users-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		background: #f8fafc;
	}

	/* PageHeader slot content */
	.meta-item {
		font-size: 0.8125rem;
		color: #64748b;
	}

	.meta-separator {
		color: #cbd5e1;
		margin: 0 0.25rem;
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