export interface DatabaseTable {
	name: string;
	rowsCount: number;
	size?: number;
	schema?: string;
	type?: 'table' | 'view' | 'system';
	createdAt?: string;
	updatedAt?: string;
}

export interface TableColumn {
	name: string;
	type: string;
	isNullable: boolean;
	defaultValue?: any;
	isPrimary: boolean;
	isUnique?: boolean;
	isIndexed?: boolean;
	references?: {
		table: string;
		column: string;
	};
}

export interface TableIndex {
	name: string;
	columns: string[];
	isUnique: boolean;
	isPrimary: boolean;
	type?: string;
}

export interface QueryResult {
	rows: any[];
	columns: TableColumn[];
	rowCount: number;
	executionTime: number;
	affectedRows?: number;
	error?: string;
}

export interface DatabaseStats {
	type: string;
	version: string;
	size: string;
	tableCount: number;
	connections?: number;
	uptime?: string;
}

export interface TableFilter {
	search?: string;
	schema?: string;
	type?: 'table' | 'view' | 'all';
}

export interface PaginationOptions {
	page: number;
	perPage: number;
	sortBy?: string;
	sortOrder?: 'asc' | 'desc';
}

export interface ExportOptions {
	format: 'csv' | 'json' | 'sql';
	table?: string;
	query?: string;
	includeHeaders?: boolean;
	limit?: number;
}

export interface SqlQuery {
	query: string;
	parameters?: any[];
	timeout?: number;
}

export interface DatabaseConnection {
	type: 'sqlite' | 'postgres' | 'mysql' | 'mongodb';
	connectionString?: string;
	host?: string;
	port?: number;
	database?: string;
	username?: string;
	password?: string;
	ssl?: boolean;
}

// Alias for backwards compatibility
export type DatabaseColumn = TableColumn;