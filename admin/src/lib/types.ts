// User types
export interface User {
	id: string;
	email: string;
	confirmed: boolean;
	role: 'user' | 'manager' | 'admin' | 'deleted';
	first_name?: string;
	last_name?: string;
	display_name?: string;
	phone?: string;
	location?: string;
	created_at: Date;
	updated_at: Date;
	metadata?: Record<string, any>;
}

// Auth types
export interface LoginRequest {
	email: string;
	password: string;
}

export interface LoginResponse {
	token: string;
	user: User;
}

export interface SignupRequest {
	email: string;
	password: string;
	metadata?: Record<string, any>;
}

// Database types
export interface DatabaseTable {
	name: string;
	schema: string;
	type: 'table' | 'view';
	rows_count: number;
	size: string;
}

export interface DatabaseColumn {
	name: string;
	type: string;
	nullable: boolean;
	default?: string;
	is_primary: boolean;
	is_unique: boolean;
}

export interface QueryResult {
	columns: string[];
	rows: any[][];
	affected_rows?: number;
	execution_time: number;
}

// Storage types
export interface StorageObject {
	id: string;
	name: string;
	bucket: string;
	size: number;
	mime_type: string;
	created_at: Date;
	updated_at: Date;
	public_url?: string;
}

export interface StorageBucket {
	id: string;
	name: string;
	public: boolean;
	created_at: Date;
	objects_count: number;
	total_size: number;
}

// Collection types
export interface Collection {
	id: string;
	name: string;
	schema: CollectionSchema;
	created_at: Date;
	updated_at: Date;
	records_count: number;
}

export interface CollectionSchema {
	fields: CollectionField[];
}

export interface CollectionField {
	name: string;
	type: 'text' | 'number' | 'boolean' | 'date' | 'select' | 'relation' | 'file' | 'json';
	required: boolean;
	unique?: boolean;
	default?: any;
	options?: any;
}

// Settings types
export interface AppSettings {
	app_name: string;
	app_url: string;
	allow_signup: boolean;
	require_email_confirmation: boolean;
	smtp_enabled: boolean;
	smtp_host?: string;
	smtp_port?: number;
	smtp_user?: string;
	storage_provider: 'local' | 's3';
	s3_bucket?: string;
	s3_region?: string;
	max_upload_size: number;
	allowed_file_types: string;
	session_timeout: number;
	password_min_length: number;
	enable_api_logs: boolean;
	enable_debug_mode: boolean;
	maintenance_mode: boolean;
	maintenance_message?: string;
	notification?: string;
}

// Dashboard types
export interface DashboardStats {
	total_users: number;
	total_collections: number;
	total_storage_used: number;
	total_api_calls: number;
	users_growth: number;
	storage_growth: number;
	recent_activities: Activity[];
}

export interface Activity {
	id: string;
	type: 'user_signup' | 'user_login' | 'collection_created' | 'file_uploaded' | 'settings_updated';
	description: string;
	user_id?: string;
	user_email?: string;
	created_at: Date;
}

// API Response types
export interface ApiResponse<T> {
	data?: T;
	error?: string;
	message?: string;
}

export interface PaginatedResponse<T> {
	data: T[];
	total: number;
	page: number;
	page_size: number;
	total_pages: number;
}