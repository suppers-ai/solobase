// Common types used across the SDK

// Re-export auto-generated database types
export * from './database';

// Import generated types for aliasing
import {
  AuthUser as GeneratedAuthUser,
  StorageStorageObject as GeneratedStorageObject,
  StorageStorageBucket as GeneratedBucket,
  IAMRole as GeneratedIAMRole
} from './database';

export interface SolobaseConfig {
  url: string;
  apiKey?: string;
  headers?: Record<string, string>;
  timeout?: number;
}

// Type alias for backward compatibility - uses generated type
export type User = GeneratedAuthUser & {
  roles?: string[];  // Additional field for runtime role names
};

export interface AuthTokens {
  access_token: string;
  refresh_token?: string;
  expires_in: number;
  token_type: string;
}

// Type alias for backward compatibility - uses generated type
export type StorageObject = GeneratedStorageObject;

// Metadata structure (stored as JSON string in StorageObject.metadata)
export interface StorageObjectMetadata {
  icon?: string;  // Emoji or icon identifier
  description?: string;  // User-provided description
  date?: string;  // Custom date field (ISO string or YYYY-MM-DD)
  order?: number;  // Sort order within parent folder
  path?: string;  // Virtual path for organization
  tags?: string[];  // User tags for categorization
  color?: string;  // Color coding
  starred?: boolean;  // Favorite/starred status
  [key: string]: any;  // Allow additional custom fields
}

// Type alias for backward compatibility - uses generated type
export type Bucket = GeneratedBucket;

export interface Collection {
  id: string;
  name: string;
  schema?: Record<string, any>;
  created_at: string;
  updated_at: string;
}

export interface QueryOptions {
  limit?: number;
  offset?: number;
  order?: string;
  filter?: Record<string, any>;
}

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  per_page: number;
}

export interface ApiResponse<T = any> {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
    details?: any;
  };
}

// Re-export IAMRole from generated types
export type IAMRole = GeneratedIAMRole;

export interface IAMPolicy {
  id?: string;
  subject: string;  // Role or user
  resource: string;
  action: string;
  effect: 'allow' | 'deny';
}

export interface IAMAuditLog {
  id: string;
  user_id: string;
  action: string;
  resource: string;
  result: 'allowed' | 'denied';
  metadata?: Record<string, any>;
  created_at: string;
}

export interface UploadOptions {
  contentType?: string;
  metadata?: Record<string, any>;
  public?: boolean;
  onProgress?: (progress: number) => void;
}

// Helper functions for StorageObject
export function isFolder(obj: StorageObject): boolean {
  return obj.content_type === 'application/x-directory';
}

export function isFile(obj: StorageObject): boolean {
  return obj.content_type !== 'application/x-directory';
}

export function parseMetadata(obj: StorageObject): StorageObjectMetadata | null {
  if (!obj.metadata) return null;
  try {
    return typeof obj.metadata === 'string' ? JSON.parse(obj.metadata) : obj.metadata;
  } catch (e) {
    console.warn('Failed to parse metadata:', e);
    return null;
  }
}

export function getDisplayName(obj: StorageObject): string {
  return obj.object_name || 'Unnamed';
}

export function getFileExtension(obj: StorageObject): string {
  if (isFolder(obj)) return '';
  const parts = obj.object_name.split('.');
  return parts.length > 1 ? parts[parts.length - 1] : '';
}