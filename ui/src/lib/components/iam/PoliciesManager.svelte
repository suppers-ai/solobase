<script>
	import { createEventDispatcher } from 'svelte';
	import PolicyTable from './PolicyTable.svelte';
	import CreatePolicyModal from './CreatePolicyModal.svelte';
	import EditPolicyModal from './EditPolicyModal.svelte';
	
	export let policies = [];
	
	const dispatch = createEventDispatcher();
	
	let showCreateModal = false;
	let showEditModal = false;
	let selectedPolicy = null;
	
	async function handleCreatePolicy(event) {
		const policy = event.detail;
		const response = await fetch('/api/admin/iam/policies', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				'Authorization': `Bearer ${localStorage.getItem('auth_token')}`
			},
			body: JSON.stringify(policy)
		});
		
		if (response.ok) {
			dispatch('policiesChanged');
			showCreateModal = false;
		} else {
			alert('Failed to create policy');
		}
	}
	
	async function handleDeletePolicy(event) {
		const policy = event.detail;
		if (!confirm('Are you sure you want to delete this policy?')) {
			return;
		}
		
		const response = await fetch('/api/admin/iam/policies', {
			method: 'DELETE',
			headers: {
				'Content-Type': 'application/json',
				'Authorization': `Bearer ${localStorage.getItem('auth_token')}`
			},
			body: JSON.stringify(policy)
		});
		
		if (response.ok) {
			dispatch('policiesChanged');
		} else {
			alert('Failed to delete policy');
		}
	}
	
	function handleEditPolicy(event) {
		selectedPolicy = event.detail;
		showEditModal = true;
	}
	
	async function handleSavePolicy(event) {
		const policy = event.detail;
		// For policies, we need to delete the old one and create a new one
		// since Casbin doesn't have a direct update method
		const deleteResponse = await fetch('/api/admin/iam/policies', {
			method: 'DELETE',
			headers: {
				'Content-Type': 'application/json',
				'Authorization': `Bearer ${localStorage.getItem('auth_token')}`
			},
			body: JSON.stringify(selectedPolicy)
		});
		
		if (deleteResponse.ok) {
			const createResponse = await fetch('/api/admin/iam/policies', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': `Bearer ${localStorage.getItem('auth_token')}`
				},
				body: JSON.stringify(policy)
			});
			
			if (createResponse.ok) {
				dispatch('policiesChanged');
				showEditModal = false;
				selectedPolicy = null;
			} else {
				alert('Failed to update policy');
			}
		} else {
			alert('Failed to update policy');
		}
	}
</script>

<div class="section">
	<div class="section-header">
		<h2>Policies</h2>
		<button class="btn btn-primary" on:click={() => showCreateModal = true}>
			Create Policy
		</button>
	</div>
	
	<PolicyTable 
		{policies}
		on:delete={handleDeletePolicy}
		on:edit={handleEditPolicy}
	/>
</div>

{#if showCreateModal}
	<CreatePolicyModal
		on:create={handleCreatePolicy}
		on:close={() => showCreateModal = false}
	/>
{/if}

{#if showEditModal}
	<EditPolicyModal
		policy={selectedPolicy}
		on:save={handleSavePolicy}
		on:close={() => {
			showEditModal = false;
			selectedPolicy = null;
		}}
	/>
{/if}

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
		align-items: center;
		margin-bottom: 2rem;
	}
	
	.section-header h2 {
		margin: 0;
		color: #333;
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
	
	.btn:hover {
		background: #f5f5f5;
	}
	
	.btn-primary {
		background: #4CAF50;
		color: white;
		border-color: #4CAF50;
	}
	
	.btn-primary:hover {
		background: #45a049;
	}
</style>