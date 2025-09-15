// Auto-generated from GORM models - DO NOT EDIT MANUALLY
// Generated at: 2025-09-15T14:27:09+12:00
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
// Package: auth
// ============================================

export interface AuthSession {
  id: string;
  user_id: string;
  token: string;
  expires_at: string | Date;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface AuthToken {
  id: string;
  user_id: string;
  token: string;
  // reset, confirm, etc.
  type: string;
  expires_at: string | Date;
  used_at?: string | Date | null;
  created_at: string | Date;
}

export interface AuthUser {
  id: string;
  email: string;
  username: string;
  confirmed: boolean;
  first_name: string;
  last_name: string;
  display_name: string;
  phone: string;
  location: string;
  last_login?: string | Date | null;
  metadata: string;
  created_at: string | Date;
  updated_at: string | Date;
  deleted_at?: string | Date | null;
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
// Package: database
// ============================================

export interface BaseModel {
  id: string;
  created_at: string | Date;
  updated_at: string | Date;
  deleted_at: any;
}

export interface Config {
  type: string;
  dsn: string;
  host: string;
  port: number;
  database: string;
  username: string;
  password: string;
  ssl_mode: string;
  max_open_conns: number;
  max_idle_conns: number;
  conn_max_lifetime: any;
  debug: boolean;
  auto_migrate: boolean;
}

export interface TimestampModel {
  id: string;
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
  // System roles cannot be deleted
  is_system: boolean;
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
// Package: logger
// ============================================

export interface Config {
  level: any;
  // console, database, file, multi
  output: string;
  // json, text
  format: string;
  buffer_size: number;
  flush_interval: any;
  max_batch_size: number;
  async_mode: boolean;
  include_stack: boolean;
  include_caller: boolean;
  enable_rotation: boolean;
  // MB for file rotation
  max_size: number;
  // days for file rotation
  max_age: number;
  // number of backup files
  max_backups: number;
  file_path: string;
  extra: Record<string, any>;
}

export interface Field {
}

export interface Log {
  id: string;
  level: any;
  message: string;
  fields: Record<string, any>;
  user_id?: string | null;
  trace_id?: string | null;
  timestamp: string | Date;
}

export interface LogFilter {
  level?: any | null;
  user_id?: string | null;
  trace_id?: string | null;
  start_time?: string | Date | null;
  end_time?: string | Date | null;
  limit: number;
  offset: number;
  order_by: string;
  order_desc: boolean;
}

export interface LogModel {
  id: string;
  level: string;
  message: string;
  fields: any;
  user_id?: string | null;
  trace_id?: string | null;
  created_at: string | Date;
}

export interface MiddlewareConfig {
}

export interface RequestLog {
  id: string;
  level: any;
  method: string;
  path: string;
  query: string;
  status_code: number;
  exec_time_ms: number;
  user_ip: string;
  user_agent: string;
  user_id?: string | null;
  trace_id?: string | null;
  error?: string | null;
  request_body?: string | null;
  response_body?: string | null;
  headers?: string | null;
  created_at: string | Date;
}

export interface RequestLogFilter {
  method?: string | null;
  path?: string | null;
  path_prefix?: string | null;
  status_code?: number | null;
  min_exec_time?: number | null;
  max_exec_time?: number | null;
  user_id?: string | null;
  user_ip?: string | null;
  trace_id?: string | null;
  has_error?: boolean | null;
  start_time?: string | Date | null;
  end_time?: string | Date | null;
  limit: number;
  offset: number;
  order_by: string;
  order_desc: boolean;
}

export interface RequestLogModel {
  id: string;
  level: string;
  method: string;
  path: string;
  query?: string | null;
  status_code: number;
  exec_time_ms: number;
  user_ip: string;
  user_agent?: string | null;
  user_id?: string | null;
  trace_id?: string | null;
  error?: string | null;
  request_body?: string | null;
  response_body?: string | null;
  headers?: string | null;
  created_at: string | Date;
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

export interface ProductFieldConstraints {
  required: boolean;
  min?: number | null;
  max?: number | null;
  min_length?: number | null;
  max_length?: number | null;
  pattern: string;
  // For select/enum types
  options: string[];
  default: any;
  placeholder: string;
}

export interface ProductFieldDefinition {
  // e.g., "filter_text_1", "filter_numeric_1"
  id: string;
  // Display name for the field
  name: string;
  // numeric, text, boolean, enum, location
  type: string;
  required: boolean;
  description: string;
  constraints: any;
}

export interface ProductGroup {
  id: number;
  user_id: number;
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
  // Custom field definitions
  fields: any[];
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
  is_active: boolean;
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
  // Custom field definitions
  fields: any[];
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
  is_active: boolean;
  created_at: string | Date;
  updated_at: string | Date;
}

// ============================================
// Package: storage
// ============================================

export interface StorageStorageBucket {
  id: string;
  name: string;
  public: boolean;
  created_at: string | Date;
  updated_at: string | Date;
}

export interface StorageStorageObject {
  id: string;
  bucket_name: string;
  // Just the name (file.txt or foldername)
  object_name: string;
  // ID of parent folder, null for root items
  parent_folder_id?: string | null;
  size: number;
  // "application/x-directory" for folders
  content_type: string;
  // MD5 or SHA256 hash
  checksum: string;
  // JSON string
  metadata: string;
  created_at: string | Date;
  updated_at: string | Date;
  // Track when the item was last viewed
  last_viewed?: string | Date | null;
  user_id: string;
  // Application ID, null for admin uploads
  app_id?: string | null;
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
