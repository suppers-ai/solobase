// Re-export all shared types
export * from '$shared/types';

// Re-export frontend-specific auth types
export { type AuthState } from './auth.types';

// Re-export frontend-specific storage types
export {
	type BucketPolicy,
	type FileUploadProgress,
	type StorageStats,
	type FileFilter,
	type FileAction,
	type UploadOptions
} from './storage.types';

// Re-export frontend-specific API types
export {
	type ApiRequest,
	type BatchRequest,
	type BatchResponse,
	type WebSocketMessage,
	type ApiClientConfig,
	type DashboardStats,
	type AppSettings
} from './api.types';

// Re-export database types
export * from './database.types';

// Re-export product-related types
export * from './products';
export * from './field-definition';
