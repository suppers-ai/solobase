// Re-export shared storage types
export type {
	StorageObject,
	Bucket,
	StorageBucket,
	StorageStorageObject,
	StorageStorageBucket,
	StorageObjectMetadata
} from '$shared/types';

export {
	isFolder,
	isFile,
	parseMetadata,
	getDisplayName,
	getFileExtension
} from '$shared/types';

// Frontend-specific storage types
import type { StorageObject } from '$shared/types';

export interface BucketPolicy {
	id: string;
	bucket_id: string;
	name: string;
	definition: Record<string, any>;
	created_at: string;
	updated_at?: string;
}

export interface FileUploadProgress {
	fileName: string;
	progress: number;
	status: 'pending' | 'uploading' | 'completed' | 'error';
	error?: string;
}

export interface StorageStats {
	totalStorage: string;
	usedStorage: string;
	availableStorage: string;
	totalFiles: number;
	totalBuckets: number;
	quotaPercentage: number;
}

export interface FileFilter {
	type?: 'all' | 'folder' | 'file' | 'image' | 'video' | 'document';
	search?: string;
	sortBy?: 'name' | 'size' | 'modified';
	sortOrder?: 'asc' | 'desc';
}

export interface FileAction {
	type: 'preview' | 'download' | 'rename' | 'delete' | 'copy' | 'move' | 'share';
	item: StorageObject;
	targetPath?: string;
}

export interface UploadOptions {
	bucket: string;
	path?: string;
	parentFolderId?: string;
	public?: boolean;
	metadata?: Record<string, any>;
	onProgress?: (progress: number) => void;
	onComplete?: (object: StorageObject) => void;
	onError?: (error: Error) => void;
}
