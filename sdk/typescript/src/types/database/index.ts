// Auto-generated from GORM models - DO NOT EDIT MANUALLY
// Generated at: 2025-09-29T22:42:09+13:00
// Run 'go run scripts/generate-types.go' to regenerate

// ============================================
// Package: analytics
// ============================================

export interface AnalyticsAnalyticsStats {
  totalViews: number;
  uniqueUsers: number;
  todayViews: number;
  activeNow: number;
}

export interface AnalyticsDailyStats {
  date: string | Date;
  page_views: number;
  unique_users: number;
  events: number;
}

export interface AnalyticsEvent {
  id: string;
  user_id?: string | null;
  event_name: string;
  event_data: any;
  created_at: string | Date;
}

export interface AnalyticsPageView {
  id: string;
  user_id?: string | null;
  session_id: string;
  page_url: string;
  referrer?: string | null;
  user_agent?: string | null;
  ip_address?: string | null;
  created_at: string | Date;
}

export interface AnalyticsPageViewStats {
  page_url: string;
  views: number;
}

// ============================================
// Package: cloudstorage
// ============================================

export interface CloudStorageRoleQuota {
  id: string;
  role_id: string;
  // Indexed for faster lookups
  role_name: string;
  // 5GB default
  max_storage_bytes: number;
  // 10GB default
  max_bandwidth_bytes: number;
  // 100MB default
  max_upload_size: number;
  // 1000 files default
  max_files_count: number;
  // Comma-separated list
  allowed_extensions: string;
  // Comma-separated list
  blocked_extensions: string;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface CloudStorageStorageAccessLog {
  id: string;
  object_id: string;
  user_id?: string | null;
  ip_address?: string | null;
  action: any;
  user_agent?: string | null;
  metadata: any;
  // Use GORM's auto create time
  created_at: string | Date;
}

export interface CloudStorageStorageQuota {
  id: string;
  user_id: string;
  // 5GB default
  max_storage_bytes: number;
  // 10GB default
  max_bandwidth_bytes: number;
  storage_used: number;
  bandwidth_used: number;
  reset_bandwidth_at?: string | Date | null;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface CloudStorageStorageShare {
  id: string;
  object_id: string;
  shared_with_user_id?: string | null;
  shared_with_email?: string | null;
  permission_level: any;
  inherit_to_children: boolean;
  share_token?: string | null;
  is_public: boolean;
  expires_at?: string | Date | null;
  created_by: string;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface CloudStorageStorageShareWithObject {
  object_name: string;
  content_type: string;
  size: number;
  object_created_at: string | Date;
  object_metadata: any;
}

export interface CloudStorageUserQuotaOverride {
  id: string;
  // Unique index for fast lookups
  user_id: string;
  max_storage_bytes?: number | null;
  max_bandwidth_bytes?: number | null;
  max_upload_size?: number | null;
  max_files_count?: number | null;
  allowed_extensions?: string | null;
  blocked_extensions?: string | null;
  // Why this override was created
  reason: string;
  // Indexed for expiry queries
  expires_at?: string | Date | null;
  // Indexed for admin queries
  created_by: string;
  created_at: string | Date;
  updated_at: string | Date;
}

// ============================================
// Package: iam
// ============================================

export interface IAMIAMAuditLog {
  id: string;
  user_id: string;
  action: string;
  resource: string;
  // "allow" or "deny"
  result: string;
  reason: string;
  ip_address: string;
  user_agent: string;
  metadata: Record<string, any>;
  created_at: string | Date;
}

export interface IAMRole {
  id: string;
  name: string;
  display_name: string;
  description: string;
  // "system" for protected roles, "custom" for user-created roles
  type: string;
  metadata: Record<string, any>;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface IAMRoleMetadata {
  // IP whitelist for access control
  allowed_ips: string[];
  // List of disabled features for this role
  disabled_features: string[];
}

export interface IAMUserRole {
  id: string;
  user_id: string;
  role_id: string;
  granted_by: string;
  granted_at: string | Date;
  expires_at?: string | Date | null;
}

// ============================================
// Package: models
// ============================================

export interface ProductAppSettings {
  app_name: string;
  app_url: string;
  allow_signup: boolean;
  require_email_confirmation: boolean;
  smtp_enabled: boolean;
  smtp_host: string;
  smtp_port: number;
  smtp_user: string;
  storage_provider: string;
  s3_bucket: string;
  s3_region: string;
  max_upload_size: number;
  allowed_file_types: string;
  // in minutes
  session_timeout: number;
  password_min_length: number;
  enable_api_logs: boolean;
  enable_debug_mode: boolean;
  maintenance_mode: boolean;
  maintenance_message: string;
  notification: string;
}

export interface ProductCustomTableDefinition {
  id: number;
  // Actual table name with custom_ prefix
  name: string;
  // User-friendly name without prefix
  display_name: string;
  description: string;
  // Column definitions
  fields: any[];
  // Index definitions
  indexes: any[];
  // Table options
  options: any;
  // User ID who created the table
  created_by: string;
  // active, disabled, archived
  status: string;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface ProductCustomTableField {
  // Column name
  name: string;
  // GORM data type: string, int, float, bool, time, json
  type: string;
  // For varchar(n)
  size: number;
  nullable: boolean;
  default_value: any;
  is_primary_key: boolean;
  is_unique: boolean;
  is_indexed: boolean;
  auto_increment: boolean;
  description: string;
  foreign_key?: any | null;
  validation: any;
}

export interface ProductCustomTableIndex {
  name: string;
  columns: string[];
  unique: boolean;
  // btree, hash, gin, gist (PostgreSQL specific)
  type: string;
}

export interface ProductCustomTableMigration {
  id: number;
  table_id: number;
  version: number;
  // create, alter, drop
  migration_type: string;
  old_schema: any;
  new_schema: any;
  executed_by: string;
  executed_at: string | Date;
  rollback_at?: string | Date | null;
  // pending, completed, failed, rolled_back
  status: string;
  error_message: string;
}

export interface ProductCustomTableOptions {
  // Add deleted_at field
  soft_delete: boolean;
  // Add created_at, updated_at
  timestamps: boolean;
  // Add version field for optimistic locking
  versioning: boolean;
  // Track changes in audit log
  auditing: boolean;
  // Enable query caching
  cache_enabled: boolean;
  // Maximum allowed rows
  max_rows: number;
}

export interface ProductFieldConstraints {
  required: boolean;
  // For numeric/range types
  min?: number | null;
  // For numeric/range types
  max?: number | null;
  // For text types
  min_length?: number | null;
  // For text types
  max_length?: number | null;
  // Regex pattern for validation
  pattern: string;
  // For select/enum types
  options: string[];
  // Default value
  default: any;
  // UI placeholder text
  placeholder: string;
  // For textarea type
  rows?: number | null;
  // For numeric/range types
  step?: number | null;
  // Whether the user can edit this field
  editable_by_user: boolean;
}

export interface ProductFieldDefinition {
  // For filter fields: "filter_text_1", etc. For custom fields: any unique ID
  id: string;
  // Display name for the field
  name: string;
  // text, numeric, boolean, enum, color, date, email, url, textarea, etc.
  type: string;
  required: boolean;
  description: string;
  // Section/tab this field belongs to (for UI organization)
  section: string;
  // Display order within section
  order: number;
  // All validation constraints
  constraints: any;
}

export interface ProductFieldValidation {
  min_length?: number | null;
  max_length?: number | null;
  min_value?: number | null;
  max_value?: number | null;
  // Regex pattern
  pattern: string;
  // Allowed values
  enum_values: string[];
  required: boolean;
}

export interface ProductForeignKeyDef {
  // Table to reference (with custom_ prefix)
  reference_table: string;
  // Column in reference table
  reference_column: string;
  // CASCADE, SET NULL, RESTRICT
  on_delete: string;
  // CASCADE, SET NULL, RESTRICT
  on_update: string;
}

export interface ProductGroup {
  id: number;
  user_id: string;
  group_template_id: number;
  group_template: any;
  name: string;
  description: string;
  filter_numeric_1?: number | null;
  filter_numeric_2?: number | null;
  filter_numeric_3?: number | null;
  filter_numeric_4?: number | null;
  filter_numeric_5?: number | null;
  filter_text_1?: string | null;
  filter_text_2?: string | null;
  filter_text_3?: string | null;
  filter_text_4?: string | null;
  filter_text_5?: string | null;
  filter_boolean_1?: boolean | null;
  filter_boolean_2?: boolean | null;
  filter_boolean_3?: boolean | null;
  filter_boolean_4?: boolean | null;
  filter_boolean_5?: boolean | null;
  filter_enum_1?: string | null;
  filter_enum_2?: string | null;
  filter_enum_3?: string | null;
  filter_enum_4?: string | null;
  filter_enum_5?: string | null;
  // Store as GeoJSON or lat,lng
  filter_location_1?: string | null;
  filter_location_2?: string | null;
  filter_location_3?: string | null;
  filter_location_4?: string | null;
  filter_location_5?: string | null;
  // Additional non-indexed fields
  custom_fields: Record<string, any>;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface ProductGroupTemplate {
  id: number;
  name: string;
  display_name: string;
  description: string;
  icon: string;
  // Filter field definitions
  filter_fields_schema: any[];
  // active, pending, deleted
  status: string;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface ProductPricingTemplate {
  id: number;
  name: string;
  display_name: string;
  description: string;
  // Formula to calculate price
  price_formula: string;
  // Formula to determine if template applies
  condition_formula: string;
  // Required variables for this template
  variables: Record<string, any>;
  category: string;
  // active, pending, deleted
  status: string;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface ProductProduct {
  id: number;
  group_id: number;
  group: any;
  product_template_id: number;
  product_template: any;
  name: string;
  description: string;
  base_price: number;
  currency: string;
  filter_numeric_1?: number | null;
  filter_numeric_2?: number | null;
  filter_numeric_3?: number | null;
  filter_numeric_4?: number | null;
  filter_numeric_5?: number | null;
  filter_text_1?: string | null;
  filter_text_2?: string | null;
  filter_text_3?: string | null;
  filter_text_4?: string | null;
  filter_text_5?: string | null;
  filter_boolean_1?: boolean | null;
  filter_boolean_2?: boolean | null;
  filter_boolean_3?: boolean | null;
  filter_boolean_4?: boolean | null;
  filter_boolean_5?: boolean | null;
  filter_enum_1?: string | null;
  filter_enum_2?: string | null;
  filter_enum_3?: string | null;
  filter_enum_4?: string | null;
  filter_enum_5?: string | null;
  // Store as GeoJSON or lat,lng
  filter_location_1?: string | null;
  filter_location_2?: string | null;
  filter_location_3?: string | null;
  filter_location_4?: string | null;
  filter_location_5?: string | null;
  // Additional non-indexed fields
  custom_fields: Record<string, any>;
  // Product-specific variable values
  variables: Record<string, any>;
  // Override formula
  pricing_formula: string;
  active: boolean;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface ProductProductTemplate {
  id: number;
  name: string;
  display_name: string;
  description: string;
  category: string;
  icon: string;
  // Filter field definitions (indexed, mapped to filter columns)
  filter_fields_schema: any[];
  // Custom field definitions (non-indexed, stored in CustomFields JSON)
  custom_fields_schema: any[];
  // IDs of pricing templates to use
  pricing_templates: number[];
  // instant, approval
  billing_mode: string;
  // one-time, recurring
  billing_type: string;
  // day, week, month, year
  billing_recurring_interval?: string | null;
  billing_recurring_interval_count?: number | null;
  // active, pending, deleted
  status: string;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface ProductSetting {
  id: string;
  key: string;
  value: string;
  // string, bool, int, json
  type: string;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface ProductStorageDownloadToken {
  id: string;
  token: string;
  file_id: string;
  bucket: string;
  // Parent folder ID (null for root)
  parent_folder_id?: string | null;
  // The file name
  object_name: string;
  user_id: string;
  file_size: number;
  bytes_served: number;
  completed: boolean;
  expires_at: string | Date;
  created_at: string | Date;
  callback_at?: string | Date | null;
  client_ip: string;
}

export interface ProductStorageUploadToken {
  id: string;
  token: string;
  bucket: string;
  // Parent folder ID (null for root)
  parent_folder_id?: string | null;
  // The file name
  object_name: string;
  user_id: string;
  // Maximum allowed file size
  max_size: number;
  // Expected content type
  content_type: string;
  bytes_uploaded: number;
  completed: boolean;
  // ID of created storage object
  object_id: string;
  expires_at: string | Date;
  created_at: string | Date;
  completed_at?: string | Date | null;
  client_ip: string;
}

export interface ProductVariable {
  id: number;
  name: string;
  display_name: string;
  // number, string, boolean
  value_type: string;
  // user, system
  type: string;
  default_value: any;
  description: string;
  // active, pending, deleted
  status: string;
  created_at: string | Date;
  updated_at: string | Date;
}

// ============================================
// Helper Types
// ============================================

export type UUID = string;
export type DateTime = string | Date;
export type NullableDateTime = DateTime | null;
export type JSONData = Record<string, any>;

// ============================================
// Table Names
// ============================================

export const TableNames = {
} as const;

export type TableName = typeof TableNames[keyof typeof TableNames];
