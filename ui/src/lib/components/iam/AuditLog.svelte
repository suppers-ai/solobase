<script>
	import { onMount } from 'svelte';
	import { formatDate, formatDuration } from '$lib/utils/formatters';
	
	export let limit = 50;
	
	let logs = [];
	let loading = false;
	let filter = '';
	let selectedType = 'all';
	
	const logTypes = [
		{ value: 'all', label: 'All Events' },
		{ value: 'permission_check', label: 'Permission Checks' },
		{ value: 'role_assigned', label: 'Role Assignments' },
		{ value: 'role_removed', label: 'Role Removals' },
		{ value: 'policy_created', label: 'Policy Created' },
		{ value: 'policy_deleted', label: 'Policy Deleted' }
	];
	
	async function loadLogs() {
		loading = true;
		try {
			const params = new URLSearchParams({
				limit: limit.toString(),
				...(filter && { filter }),
				...(selectedType !== 'all' && { type: selectedType })
			});
			
			const token = localStorage.getItem('auth_token');
			console.log('AuditLog: Using token:', token ? `${token.substring(0, 20)}...` : 'null');
			
			const response = await window.fetch(`/api/admin/iam/audit-logs?${params}`, {
				method: 'GET',
				headers: {
					'Authorization': `Bearer ${token}`,
					'Content-Type': 'application/json'
				},
				credentials: 'same-origin'
			});
			
			console.log('AuditLog: Response status:', response.status);
			
			if (response.ok) {
				logs = await response.json();
				console.log('AuditLog: Loaded', logs.length, 'logs');
			} else {
				const error = await response.text();
				console.error('AuditLog: Error response:', response.status, error);
			}
		} catch (error) {
			console.error('Failed to load audit logs:', error);
		} finally {
			loading = false;
		}
	}
	
	function getEventIcon(type) {
		switch(type) {
			case 'permission_check': return '🔍';
			case 'role_assigned': return '➕';
			case 'role_removed': return '➖';
			case 'policy_created': return '📝';
			case 'policy_deleted': return '🗑️';
			default: return '📋';
		}
	}
	
	function getEventColor(type) {
		switch(type) {
			case 'permission_check': return 'blue';
			case 'role_assigned': return 'green';
			case 'role_removed': return 'orange';
			case 'policy_created': return 'purple';
			case 'policy_deleted': return 'red';
			default: return 'gray';
		}
	}
	
	onMount(() => {
		loadLogs();
	});
	
	$: if (selectedType) {
		loadLogs();
	}
</script>

<div class="section">
	<div class="section-header">
		<div>
			<h2>Audit Log</h2>
			<p class="section-subtitle">Recent IAM activity and permission checks</p>
		</div>
		
		<button class="btn" on:click={loadLogs} disabled={loading}>
			{loading ? 'Refreshing...' : 'Refresh'}
		</button>
	</div>
	
	<div class="controls">
		<div class="filter-group">
			<input 
				type="text" 
				placeholder="Filter logs..." 
				bind:value={filter}
				on:keyup={(e) => e.key === 'Enter' && loadLogs()}
			/>
			
			<select bind:value={selectedType}>
				{#each logTypes as type}
					<option value={type.value}>{type.label}</option>
				{/each}
			</select>
		</div>
	</div>
	
	<div class="logs-container">
		{#if loading}
			<div class="loading">Loading audit logs...</div>
		{:else if logs.length === 0}
			<div class="empty">No audit logs found</div>
		{:else}
			<div class="logs-list">
				{#each logs as log}
					<div class="log-entry {getEventColor(log.type)}">
						<div class="log-icon">
							{getEventIcon(log.type)}
						</div>
						
						<div class="log-content">
							<div class="log-header">
								<span class="log-type">{log.type.replace(/_/g, ' ')}</span>
								<span class="log-time">{formatDate(log.timestamp)}</span>
							</div>
							
							<div class="log-details">
								<div class="log-user">
									<strong>User:</strong> {log.user_id || 'System'}
								</div>
								
								{#if log.resource}
									<div class="log-resource">
										<strong>Resource:</strong> <code>{log.resource}</code>
									</div>
								{/if}
								
								{#if log.action}
									<div class="log-action">
										<strong>Action:</strong> <code>{log.action}</code>
									</div>
								{/if}
								
								{#if log.result !== undefined}
									<div class="log-result">
										<strong>Result:</strong> 
										<span class="result-badge" class:allowed={log.result} class:denied={!log.result}>
											{log.result ? 'Allowed' : 'Denied'}
										</span>
									</div>
								{/if}
								
								{#if log.metadata}
									<div class="log-metadata">
										<strong>Details:</strong> {JSON.stringify(log.metadata)}
									</div>
								{/if}
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	.section {
		background: white;
		border-radius: 8px;
		padding: 2rem;
		box-shadow: 0 2px 4px rgba(0,0,0,0.1);
	}
	
	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 2rem;
	}
	
	.section-header h2 {
		margin: 0 0 0.5rem;
		color: #333;
	}
	
	.section-subtitle {
		margin: 0;
		color: #666;
		font-size: 0.9rem;
	}
	
	.controls {
		margin-bottom: 1.5rem;
	}
	
	.filter-group {
		display: flex;
		gap: 1rem;
	}
	
	.filter-group input {
		flex: 1;
		padding: 0.5rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 1rem;
	}
	
	.filter-group select {
		padding: 0.5rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 1rem;
		background: white;
	}
	
	.logs-container {
		max-height: 600px;
		overflow-y: auto;
		border: 1px solid #e0e0e0;
		border-radius: 8px;
	}
	
	.loading,
	.empty {
		padding: 3rem;
		text-align: center;
		color: #666;
	}
	
	.logs-list {
		padding: 1rem;
	}
	
	.log-entry {
		display: flex;
		gap: 1rem;
		padding: 1rem;
		margin-bottom: 1rem;
		background: #f9f9f9;
		border-radius: 8px;
		border-left: 3px solid;
	}
	
	.log-entry.blue {
		border-left-color: #2196F3;
	}
	
	.log-entry.green {
		border-left-color: #4CAF50;
	}
	
	.log-entry.orange {
		border-left-color: #FF9800;
	}
	
	.log-entry.purple {
		border-left-color: #9C27B0;
	}
	
	.log-entry.red {
		border-left-color: #f44336;
	}
	
	.log-entry.gray {
		border-left-color: #9E9E9E;
	}
	
	.log-icon {
		font-size: 1.5rem;
		line-height: 1;
	}
	
	.log-content {
		flex: 1;
	}
	
	.log-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}
	
	.log-type {
		font-weight: 600;
		text-transform: capitalize;
		color: #333;
	}
	
	.log-time {
		font-size: 0.85rem;
		color: #666;
	}
	
	.log-details {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
		font-size: 0.9rem;
	}
	
	.log-details > div {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	
	.log-details strong {
		color: #666;
	}
	
	.log-details code {
		background: #e0e0e0;
		padding: 0.2rem 0.4rem;
		border-radius: 3px;
		font-family: monospace;
	}
	
	.result-badge {
		display: inline-block;
		padding: 0.2rem 0.5rem;
		border-radius: 3px;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
	}
	
	.result-badge.allowed {
		background: #d4edda;
		color: #155724;
	}
	
	.result-badge.denied {
		background: #f8d7da;
		color: #721c24;
	}
	
	.btn {
		padding: 0.5rem 1rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		background: white;
		color: #333;
		cursor: pointer;
		transition: all 0.3s;
	}
	
	.btn:hover:not(:disabled) {
		background: #f5f5f5;
	}
	
	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>