// Re-export all shared types from the @solobase/types package
export * from '@solobase/types';

// SDK-specific types

export interface SolobaseConfig {
	url: string;
	apiKey?: string;
	headers?: Record<string, string>;
	timeout?: number;
}

export interface Collection {
	id: string;
	name: string;
	schema?: Record<string, any>;
	createdAt: string;
	updatedAt: string;
}

export interface QueryOptions {
	limit?: number;
	offset?: number;
	order?: string;
	filter?: Record<string, any>;
}

// SDK-specific upload options (different from frontend's UploadOptions)
export interface UploadOptions {
	contentType?: string;
	metadata?: Record<string, any>;
	public?: boolean;
	onProgress?: (progress: number) => void;
}
