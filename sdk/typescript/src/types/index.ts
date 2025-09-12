// Common types used across the SDK

export interface SolobaseConfig {
  url: string;
  apiKey?: string;
  headers?: Record<string, string>;
  timeout?: number;
}

export interface User {
  id: string;
  email: string;
  role: 'user' | 'admin' | 'manager';
  confirmed: boolean;
  created_at: string;
  updated_at: string;
  metadata?: Record<string, any>;
}

export interface AuthTokens {
  access_token: string;
  refresh_token?: string;
  expires_in: number;
  token_type: string;
}

// Matches the Go StorageObject struct exactly
export interface StorageObject {
  id: string;
  bucket_name: string;
  object_name: string;  // Just the name (file.txt or foldername)
  parent_folder_id?: string | null;  // ID of parent folder, null for root items
  size: number;
  content_type: string;  // "application/x-directory" for folders
  checksum?: string;
  metadata?: string;  // JSON string containing metadata
  created_at: string | Date;
  updated_at: string | Date;
  last_viewed?: string | Date | null;
  user_id?: string;
  app_id?: string | null;
}

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

export interface Bucket {
  id: string;
  name: string;
  public: boolean;
  created_at: string;
  updated_at: string;
}

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