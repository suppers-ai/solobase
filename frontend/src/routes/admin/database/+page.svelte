<script lang="ts">
	import { onMount } from "svelte";
	import {
		Database,
		ChevronDown,
		RefreshCw,
		Download,
		Upload,
		Table,
		Code,
		ChevronLeft,
		ChevronRight,
		Server,
		AlertCircle,
		CheckCircle,
		FileText,
		Folder,
		FolderOpen,
	} from "lucide-svelte";
	import SearchInput from "$lib/components/SearchInput.svelte";
	import { api } from "$lib/api";
	import ExportButton from "$lib/components/ExportButton.svelte";
	import LoadingSpinner from "$lib/components/ui/LoadingSpinner.svelte";
	import EmptyState from "$lib/components/ui/EmptyState.svelte";
	import TabNavigation from "$lib/components/ui/TabNavigation.svelte";

	const databaseTabs = [
		{ id: 'table', label: 'Table Browser', icon: Table },
		{ id: 'sql', label: 'SQL Console', icon: Code }
	];
	import { requireAdmin } from "$lib/utils/auth";

	interface TableInfo {
		name?: string;
		value?: string;
	}

	interface TablesResponse {
		data?: TableInfo[];
		tables?: TableInfo[];
	}

	interface QueryResult {
		rows?: unknown[][];
		columns?: string[];
		error?: string;
		affectedRows?: number;
	}

	interface DatabaseInfo {
		type?: string;
		version?: string;
	}

	let selectedTable = "";
	let activeTab = "table";
	let searchQuery = "";
	let currentPage = 1;
	let totalPages = 1;
	let totalRows = 0;
	let rowsPerPage = 25;
	let loading = false;
	let dbType = "SQLite";
	let dbVersion = "";
	let dbSize = "";
	let dropdownOpen = false;
	let expandedGroups: Set<string> = new Set();

	// Real data from API
	let tableData: any[] = [];
	let tables: any[] = [];
	let tableColumns: any[] = [];
	let groupedTables: Map<string, any[]> = new Map();

	let sqlQuery = `SELECT * FROM auth_users ORDER BY created_at DESC LIMIT 10;`;
	let sqlResults: any[] = [];
	let sqlError = "";
	let sqlExecuting = false;
	let queryExecutionTime = 0;
	let affectedRows = 0;

	function groupTablesByPrefix(tableList: any[]) {
		const groups = new Map<string, any[]>();

		tableList.forEach(table => {
			const name = table.name || table.value;
			let groupName = "Other";

			// Check for common prefixes
			if (name.startsWith("auth_")) {
				groupName = "Authentication";
			} else if (name.startsWith("iam_")) {
				groupName = "IAM (Identity & Access)";
			} else if (name.startsWith("ext_")) {
				groupName = "Extensions";
			} else if (name.startsWith("analytics_")) {
				groupName = "Analytics";
			} else if (name.startsWith("log_") || name.startsWith("logs_")) {
				groupName = "Logs";
			} else if (name.startsWith("storage_")) {
				groupName = "Storage";
			} else if (name.includes("_")) {
				// Try to extract a meaningful prefix
				const prefix = name.split("_")[0];
				groupName = prefix.charAt(0).toUpperCase() + prefix.slice(1);
			}

			if (!groups.has(groupName)) {
				groups.set(groupName, []);
			}
			groups.get(groupName)!.push(table);
		});

		// Sort tables within each group
		groups.forEach((tables, key) => {
			tables.sort((a, b) => (a.name || a.value).localeCompare(b.name || b.value));
		});

		return groups;
	}

	function toggleGroup(groupName: string) {
		if (expandedGroups.has(groupName)) {
			expandedGroups.delete(groupName);
		} else {
			expandedGroups.add(groupName);
		}
		expandedGroups = new Set(expandedGroups);
	}

	function selectTable(tableName: string) {
		selectedTable = tableName;
		dropdownOpen = false;
		currentPage = 1;
		totalRows = 0;
		loadTableData();
	}

	async function handleTableChange(e: Event) {
		selectedTable = (e.target as HTMLSelectElement).value;
		currentPage = 1;
		totalRows = 0;
		await loadTableData();
	}

	async function loadTables() {
		loading = true;
		try {
			const response = await api.get<TableInfo[] | TablesResponse>("/admin/database/tables");
			// Ensure tables is always an array
			if (Array.isArray(response)) {
				tables = response;
			} else if (response && typeof response === "object") {
				const typedResponse = response as TablesResponse;
				tables = typedResponse.data || typedResponse.tables || [];
			} else {
				tables = [];
			}

			// Group tables by prefix
			groupedTables = groupTablesByPrefix(tables);

			// Auto-expand groups initially
			groupedTables.forEach((_, groupName) => {
				expandedGroups.add(groupName);
			});

			// Select users table by default if available
			if (tables.length > 0 && !selectedTable) {
				const usersTable = tables.find(
					(t) =>
						t.name === "users" ||
						t.name === "auth_users" ||
						t.value === "users" ||
						t.value === "auth_users",
				);

				if (usersTable) {
					selectedTable = usersTable.name || usersTable.value;
				} else {
					selectedTable = tables[0].name || tables[0].value;
				}

				await loadTableData();
			}
		} catch (error) {
			console.error("Failed to load tables:", error);
			tables = [];
		} finally {
			loading = false;
		}
	}

	async function loadTableData() {
		if (!selectedTable) return;

		loading = true;
		tableData = [];
		tableColumns = [];

		try {
			// Get table columns
			const columns = await api.get<string[]>(
				`/admin/database/tables/${selectedTable}/columns`,
			);
			tableColumns = columns || [];

			// Get total count of rows
			const countQuery = `SELECT COUNT(*) as count FROM ${selectedTable}`;
			const countResult = await api.post<QueryResult>("/admin/database/query", {
				query: countQuery,
			});
			if (countResult.rows && countResult.rows[0]) {
				totalRows = (countResult.rows[0][0] as number) || 0;
				totalPages = Math.max(1, Math.ceil(totalRows / rowsPerPage));
			} else {
				totalRows = 0;
				totalPages = 1;
			}

			// Execute a SELECT query to get data
			const query = `SELECT * FROM ${selectedTable} LIMIT ${rowsPerPage} OFFSET ${(currentPage - 1) * rowsPerPage}`;
			const result = await api.post<QueryResult>("/admin/database/query", { query });

			if (result.rows && result.columns) {
				// Transform rows array to objects
				tableData = result.rows.map((row: unknown[]) => {
					const obj: Record<string, unknown> = {};
					result.columns!.forEach((col: string, index: number) => {
						obj[col] = row[index];
					});
					return obj;
				});
			} else {
				tableData = [];
			}
		} catch (error) {
			console.error("Failed to load table data:", error);
			tableData = [];
			tableColumns = [];
		} finally {
			loading = false;
		}
	}

	async function runQuery() {
		sqlError = "";
		sqlResults = [];
		affectedRows = 0;
		sqlExecuting = true;

		try {
			const startTime = Date.now();
			const result = await api.post<QueryResult>("/admin/database/query", {
				query: sqlQuery,
			});
			queryExecutionTime = Date.now() - startTime;

			if (!result) {
				sqlError = "No response from server";
			} else if (result.error) {
				sqlError = result.error;
			} else if (
				result.rows !== undefined &&
				result.columns !== undefined
			) {
				// Transform rows array to objects for display
				if (
					Array.isArray(result.rows) &&
					Array.isArray(result.columns)
				) {
					sqlResults = result.rows.map((row: unknown[]) => {
						const obj: Record<string, unknown> = {};
						result.columns!.forEach((col: string, index: number) => {
							obj[col] = row[index];
						});
						return obj;
					});
				}
			} else if (result.affectedRows !== undefined) {
				// For INSERT/UPDATE/DELETE queries
				affectedRows = result.affectedRows;
				sqlResults = [];
			} else {
				// Check if it's already formatted data
				if (Array.isArray(result)) {
					sqlResults = result;
				} else if (typeof result === "object") {
					const possibleArrays = Object.values(result).filter((v) =>
						Array.isArray(v),
					);
					if (possibleArrays.length > 0) {
						sqlResults = possibleArrays[0] as any[];
					}
				}
			}
		} catch (error: any) {
			sqlError = error.message || "Query execution failed";
		} finally {
			sqlExecuting = false;
		}
	}

	async function refreshData() {
		if (selectedTable) {
			await loadTableData();
		} else {
			await loadTables();
		}
	}

	async function goToPage(page: number) {
		if (page >= 1 && page <= totalPages) {
			currentPage = page;
			await loadTableData();
		}
	}

	async function getDatabaseInfo() {
		try {
			const info = await api.get<DatabaseInfo>("/admin/database/info");
			if (info) {
				dbType = info.type || "SQLite";
				dbVersion = info.version || "3.x";
			}
		} catch (error) {
			console.error("Failed to get database info:", error);
			dbType = "SQLite";
			dbVersion = "3.x";
		}
	}

	onMount(() => {
		if (!requireAdmin()) return;

		const init = async () => {
			await loadTables();
			await getDatabaseInfo();
		};

		init();

		// Close dropdown when clicking outside
		const handleClickOutside = (event: MouseEvent) => {
			const target = event.target as HTMLElement;
			if (!target.closest('.table-select-wrapper')) {
				dropdownOpen = false;
			}
		};

		document.addEventListener('click', handleClickOutside);
		return () => {
			document.removeEventListener('click', handleClickOutside);
		};
	});
</script>

<div class="database-page">
	<!-- Header -->
	<div class="page-header">
		<div class="header-content">
			<div class="header-left">
				<div class="header-title">
					<Database size={24} />
					<h1>Database Manager</h1>
				</div>
				<div class="header-meta">
					<span class="meta-item">{dbType}</span>
					<span class="meta-separator">•</span>
					<span class="meta-item">Version {dbVersion}</span>
					<span class="meta-separator">•</span>
					<span class="meta-item">{tables.length} tables</span>
				</div>
			</div>
			<div class="header-info">
				<span class="info-item">
					<Server size={14} />
					Connected
				</span>
			</div>
		</div>
	</div>

	<div class="content-area">

	<div class="card">
		<!-- Tabs -->
		<div class="tabs-container">
			<TabNavigation tabs={databaseTabs} bind:activeTab variant="default" />
		</div>

		<!-- Content -->
		<div class="inner-content-area">
			{#if activeTab === "table"}
				<!-- Table View -->
				<div class="table-view">
					<!-- Controls Bar -->
					<div class="controls-bar">
						<div class="controls-left">
							<div class="table-select-wrapper">
								<button
									class="table-dropdown-trigger"
									on:click={() => (dropdownOpen = !dropdownOpen)}
									disabled={loading || tables.length === 0}
								>
									<Database size={16} />
									<span class="selected-table-name">
										{selectedTable || "Select a table"}
									</span>
									<ChevronDown
										size={16}
										class="dropdown-chevron {dropdownOpen ? 'open' : ''}"
									/>
								</button>

								{#if dropdownOpen}
									<div class="table-dropdown-menu">
										{#if tables.length === 0}
											<div class="dropdown-empty">
												No tables available
											</div>
										{:else}
											{#each [...groupedTables.entries()] as [groupName, groupTables]}
												<div class="table-group">
													<button
														class="group-header"
														on:click={() => toggleGroup(groupName)}
													>
														{#if expandedGroups.has(groupName)}
															<FolderOpen size={14} />
														{:else}
															<Folder size={14} />
														{/if}
														<span class="group-name">{groupName}</span>
														<span class="group-count">({groupTables.length})</span>
													</button>

													{#if expandedGroups.has(groupName)}
														<div class="group-tables">
															{#each groupTables as table}
																<button
																	class="table-item {selectedTable === table.name ? 'selected' : ''}"
																	on:click={() => selectTable(table.name)}
																>
																	<Table size={12} />
																	<span class="table-name">{table.name}</span>
																	{#if table.rowsCount !== undefined}
																		<span class="table-rows">{table.rowsCount} rows</span>
																	{/if}
																</button>
															{/each}
														</div>
													{/if}
												</div>
											{/each}
										{/if}
									</div>
								{/if}
							</div>

							<SearchInput bind:value={searchQuery} placeholder="Search files..." maxWidth="280px" />
						</div>

						<div class="controls-right">
							<button
								class="btn-icon"
								on:click={refreshData}
								disabled={loading}
								title="Refresh"
							>
								<RefreshCw
									size={18}
									class={loading ? "spinning" : ""}
								/>
							</button>

							<ExportButton
								data={tableData}
								filename={selectedTable || "table_data"}
								disabled={tableData.length === 0}
							/>
						</div>
					</div>

					<!-- Table Container -->
					<div class="table-wrapper">
						{#if loading}
							<div class="loading-state">
								<LoadingSpinner size="lg" />
								<p>Loading table data...</p>
							</div>
						{:else if tableData.length > 0}
							<div class="table-scroll">
								<table class="data-table">
									<thead>
										<tr>
											{#each Object.keys(tableData[0]) as column}
												<th>{column}</th>
											{/each}
										</tr>
									</thead>
									<tbody>
										{#each tableData as row}
											<tr>
												{#each Object.entries(row) as [key, value]}
													<td
														title={value === null
															? "NULL"
															: typeof value ===
																  "object"
																? JSON.stringify(
																		value,
																	)
																: String(value)}
													>
														{#if value === null}
															<span class="null"
																>NULL</span
															>
														{:else if typeof value === "boolean"}
															<span
																class="boolean {value
																	? 'true'
																	: 'false'}"
															>
																{value}
															</span>
														{:else if typeof value === "object"}
															<span class="json"
																>{JSON.stringify(
																	value,
																)}</span
															>
														{:else}
															<span class="value"
																>{value}</span
															>
														{/if}
													</td>
												{/each}
											</tr>
										{/each}
									</tbody>
								</table>
							</div>

							<!-- Pagination -->
							<div class="pagination-bar">
								<div class="pagination-info">
									{#if totalRows > 0}
										Showing {Math.min(
											(currentPage - 1) * rowsPerPage + 1,
											totalRows,
										)} - {Math.min(
											currentPage * rowsPerPage,
											totalRows,
										)} of {totalRows} rows
									{:else}
										No rows to display
									{/if}
								</div>

								<div class="pagination-controls">
									<button
										class="page-btn"
										disabled={currentPage === 1}
										on:click={() =>
											goToPage(currentPage - 1)}
									>
										<ChevronLeft size={16} />
									</button>

									<span class="page-numbers">
										Page {currentPage} of {totalPages}
									</span>

									<button
										class="page-btn"
										disabled={currentPage === totalPages}
										on:click={() =>
											goToPage(currentPage + 1)}
									>
										<ChevronRight size={16} />
									</button>
								</div>
							</div>
						{:else}
							<EmptyState
								icon={Database}
								title="No data available"
								message="This table is empty or no table is selected"
							/>
						{/if}
					</div>
				</div>
			{:else}
				<!-- SQL Editor -->
				<div class="sql-view">
					<div class="sql-header">
						<div class="sql-title">
							<h2>SQL Console</h2>
							<p>Execute queries directly on your database</p>
						</div>
						<button
							class="run-button {sqlExecuting ? 'executing' : ''}"
							on:click={runQuery}
							disabled={sqlExecuting || !sqlQuery.trim()}
						>
							{#if sqlExecuting}
								<RefreshCw size={18} class="spinning" />
								<span>Executing...</span>
							{:else}
								<span>Run Query</span>
								<span class="shortcut">Ctrl+Enter</span>
							{/if}
						</button>
					</div>

					<div class="sql-editor-wrapper">
						<textarea
							class="sql-editor"
							bind:value={sqlQuery}
							placeholder="Enter your SQL query here..."
							disabled={sqlExecuting}
							spellcheck="false"
							on:keydown={(e) => {
								if (
									(e.ctrlKey || e.metaKey) &&
									e.key === "Enter"
								) {
									e.preventDefault();
									if (!sqlExecuting && sqlQuery.trim()) {
										runQuery();
									}
								}
								if (e.key === "Tab") {
									e.preventDefault();
									const start =
										e.currentTarget.selectionStart;
									const end = e.currentTarget.selectionEnd;
									const newValue =
										sqlQuery.substring(0, start) +
										"  " +
										sqlQuery.substring(end);
									sqlQuery = newValue;
									setTimeout(() => {
										e.currentTarget.selectionStart =
											e.currentTarget.selectionEnd =
												start + 2;
									}, 0);
								}
							}}
						/>
					</div>

					{#if queryExecutionTime > 0 && !sqlError}
						<div class="result-message success">
							<CheckCircle size={18} />
							{#if affectedRows > 0}
								Query executed successfully • {affectedRows} rows
								affected • {queryExecutionTime}ms
							{:else if sqlResults.length > 0}
								Query executed successfully • {sqlResults.length}
								rows returned • {queryExecutionTime}ms
							{:else}
								Query executed successfully • {queryExecutionTime}ms
							{/if}
						</div>
					{/if}

					{#if sqlError}
						<div class="result-message error">
							<AlertCircle size={18} />
							{sqlError}
						</div>
					{/if}

					{#if sqlResults.length > 0}
						<div class="results-section">
							<div class="results-header">
								<h3>Results ({sqlResults.length} rows)</h3>
								<ExportButton
									data={sqlResults}
									filename="query_results"
									disabled={sqlResults.length === 0}
								/>
							</div>

							<div class="table-scroll">
								<table class="data-table">
									<thead>
										<tr>
											{#each Object.keys(sqlResults[0]) as column}
												<th>{column}</th>
											{/each}
										</tr>
									</thead>
									<tbody>
										{#each sqlResults as row}
											<tr>
												{#each Object.values(row) as value}
													<td
														title={value === null
															? "NULL"
															: typeof value ===
																  "object"
																? JSON.stringify(
																		value,
																	)
																: String(value)}
													>
														{#if value === null}
															<span class="null"
																>NULL</span
															>
														{:else if typeof value === "boolean"}
															<span
																class="boolean {value
																	? 'true'
																	: 'false'}"
															>
																{value}
															</span>
														{:else if typeof value === "object"}
															<span class="json"
																>{JSON.stringify(
																	value,
																)}</span
															>
														{:else}
															<span class="value"
																>{value}</span
															>
														{/if}
													</td>
												{/each}
											</tr>
										{/each}
									</tbody>
								</table>
							</div>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>
</div>

<style>
	.database-page {
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
		color: #22c55e;
		font-weight: 500;
	}

	/* Tabs */
	.tabs-container {
		background: white;
		padding: 0 2rem;
	}

	/* Content Area */
	.content-area {
		flex: 1;
		padding: 0.75rem 1.5rem 1.5rem;
		overflow: auto;
	}
	.inner-content-area {
		padding: 0 0.5rem;
	}

	/* Table View */
	.table-view {
		background: white;
		display: flex;
		flex-direction: column;
		height: 100%;
		padding: 1.5rem;
	}

	/* Controls Bar */
	.controls-bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0 0 1rem 0;
		background: white;
	}

	.controls-left {
		display: flex;
		gap: 0.75rem;
		align-items: center;
	}

	.controls-right {
		display: flex;
		gap: 0.75rem;
	}

	.table-select-wrapper {
		position: relative;
	}

	.table-dropdown-trigger {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		border: 1px solid #e2e8f0;
		border-radius: 0.375rem;
		background: white;
		font-size: 0.8125rem;
		color: #475569;
		min-width: 220px;
		cursor: pointer;
		transition: all 0.15s;
	}

	.table-dropdown-trigger:hover:not(:disabled) {
		border-color: #cbd5e1;
		background-color: #f8fafc;
	}

	.table-dropdown-trigger:focus {
		outline: none;
		border-color: #3b82f6;
		box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.08);
	}

	.table-dropdown-trigger:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.selected-table-name {
		flex: 1;
		text-align: left;
		font-weight: 500;
	}

	.dropdown-chevron {
		transition: transform 0.2s;
		color: #94a3b8;
	}

	.dropdown-chevron.open {
		transform: rotate(180deg);
	}

	.table-dropdown-menu {
		position: absolute;
		top: calc(100% + 4px);
		left: 0;
		min-width: 280px;
		max-width: 400px;
		max-height: 400px;
		overflow-y: auto;
		background: white;
		border: 1px solid #e2e8f0;
		border-radius: 0.5rem;
		box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
		z-index: 50;
	}

	.dropdown-empty {
		padding: 1rem;
		text-align: center;
		color: #94a3b8;
		font-size: 0.875rem;
	}

	.table-group {
		border-bottom: 1px solid #f1f5f9;
	}

	.table-group:last-child {
		border-bottom: none;
	}

	.group-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.625rem 0.75rem;
		background: #f8fafc;
		border: none;
		color: #475569;
		font-size: 0.8125rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s;
		text-align: left;
	}

	.group-header:hover {
		background: #f1f5f9;
	}

	.group-name {
		flex: 1;
	}

	.group-count {
		color: #94a3b8;
		font-weight: 400;
		font-size: 0.75rem;
	}

	.group-tables {
		padding: 0.25rem 0;
	}

	.table-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.5rem 0.75rem;
		padding-left: 2.25rem;
		background: white;
		border: none;
		color: #64748b;
		font-size: 0.8125rem;
		cursor: pointer;
		transition: all 0.15s;
		text-align: left;
	}

	.table-item:hover {
		background: #f8fafc;
		color: #334155;
	}

	.table-item.selected {
		background: #eff6ff;
		color: #1e40af;
		font-weight: 500;
	}

	.table-name {
		flex: 1;
	}

	.table-rows {
		font-size: 0.75rem;
		color: #94a3b8;
	}

	.table-item.selected .table-rows {
		color: #60a5fa;
	}

	.btn-icon {
		padding: 0.5rem;
		border: 1px solid #e2e8f0;
		background: white;
		border-radius: 0.375rem;
		color: #64748b;
		cursor: pointer;
		transition: all 0.15s;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.btn-icon:hover:not(:disabled) {
		background: #f8fafc;
		border-color: #cbd5e1;
		color: #475569;
	}

	.btn-icon:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* Table */
	.table-wrapper {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
	}

	.table-scroll {
		flex: 1;
		overflow: auto;
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
		max-width: 300px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.data-table tbody tr:hover {
		background: #f8fafc;
	}

	.data-table .null {
		color: #94a3b8;
		font-style: italic;
		font-size: 0.8125rem;
	}

	.data-table .boolean {
		padding: 0.25rem 0.625rem;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		font-weight: 500;
		display: inline-block;
	}

	.data-table .boolean.true {
		background: #dcfce7;
		color: #15803d;
	}

	.data-table .boolean.false {
		background: #fef3c7;
		color: #a16207;
	}

	.data-table .json {
		font-family: "SF Mono", Monaco, monospace;
		font-size: 0.8125rem;
		color: #6366f1;
		display: inline-block;
		max-width: 100%;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.data-table .value {
		color: #1e293b;
		display: inline-block;
		max-width: 100%;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	/* Pagination */
	.pagination-bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem 0;
		border-top: none;
		background: white;
	}

	.pagination-info {
		font-size: 0.875rem;
		color: #64748b;
	}

	.pagination-controls {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.page-btn {
		padding: 0.5rem;
		border: 1px solid #cbd5e1;
		background: white;
		border-radius: 0.375rem;
		color: #475569;
		cursor: pointer;
		transition: all 0.2s;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.page-btn:hover:not(:disabled) {
		background: #f8fafc;
		border-color: #94a3b8;
	}

	.page-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.page-numbers {
		font-size: 0.875rem;
		color: #334155;
		font-weight: 500;
		padding: 0 0.75rem;
	}

	/* SQL View */
	.sql-view {
		background: white;
		padding: 1.5rem;
	}

	.sql-header {
		display: flex;
		justify-content: space-between;
		align-items: start;
		margin-bottom: 1.5rem;
	}

	.sql-title h2 {
		font-size: 1.125rem;
		font-weight: 600;
		color: #1e293b;
		margin: 0 0 0.25rem 0;
	}

	.sql-title p {
		font-size: 0.875rem;
		color: #64748b;
		margin: 0;
	}

	.run-button {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.625rem 1.25rem;
		background: #3b82f6;
		color: white;
		border: none;
		border-radius: 0.5rem;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.run-button:hover:not(:disabled) {
		background: #2563eb;
	}

	.run-button:disabled {
		background: #94a3b8;
		cursor: not-allowed;
	}

	.run-button.executing {
		background: #64748b;
	}

	.shortcut {
		padding: 0.125rem 0.375rem;
		background: rgba(255, 255, 255, 0.2);
		border-radius: 0.25rem;
		font-size: 0.75rem;
	}

	.sql-editor-wrapper {
		margin-bottom: 1.5rem;
	}

	.sql-editor {
		width: 100%;
		min-height: 250px;
		padding: 1.25rem;
		border: 1px solid #cbd5e1;
		border-radius: 0.5rem;
		font-family: "SF Mono", Monaco, monospace;
		font-size: 0.875rem;
		line-height: 1.6;
		resize: vertical;
		background: #f8fafc;
		transition: all 0.2s;
	}

	.sql-editor:focus {
		outline: none;
		border-color: #3b82f6;
		box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
		background: white;
	}

	/* Result Messages */
	.result-message {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.875rem 1rem;
		border-radius: 0.5rem;
		font-size: 0.875rem;
		margin-bottom: 1.5rem;
	}

	.result-message.success {
		background: #dcfce7;
		color: #15803d;
		border: 1px solid #86efac;
	}

	.result-message.error {
		background: #fee2e2;
		color: #dc2626;
		border: 1px solid #fca5a5;
	}

	/* Results Section */
	.results-section {
		margin-top: 2rem;
		padding-top: 2rem;
		border-top: 1px solid #e2e8f0;
	}

	.results-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.results-header h3 {
		font-size: 1rem;
		font-weight: 600;
		color: #1e293b;
		margin: 0;
	}

	/* States */
	.loading-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 5rem 2rem;
		color: #64748b;
		gap: 1rem;
	}

	/* Animations */
	.spinning {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	/* Responsive */
	@media (max-width: 768px) {
		.content-area {
			padding: 1rem;
		}

		.controls-bar {
			flex-direction: column;
			gap: 1rem;
			align-items: stretch;
		}

		.controls-left {
			flex-direction: column;
			width: 100%;
		}

		.table-select,
		.search-input {
			width: 100%;
		}

		.sql-header {
			flex-direction: column;
			gap: 1rem;
		}

		.run-button {
			width: 100%;
			justify-content: center;
		}

		.data-table {
			font-size: 0.8125rem;
		}

		.data-table th,
		.data-table td {
			padding: 0.75rem 1rem;
		}
	}
</style>
