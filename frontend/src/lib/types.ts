// Re-export all types from the types directory
// This file exists for backwards compatibility with imports from './types'

export * from './types/index';

// Legacy navigation types (keep for now as they're frontend-specific)
import type { ComponentType } from 'svelte';

export interface NavigationItem {
	title: string;
	href?: string;
	icon: ComponentType;
	expandable?: boolean;
	children?: NavigationSubItem[];
}

export interface NavigationSubItem {
	title: string;
	href: string;
	icon?: ComponentType;
	badge?: string;
	badgeColor?: string;
}

export interface Activity {
	id: string;
	type: 'userSignup' | 'userLogin' | 'collectionCreated' | 'fileUploaded' | 'settingsUpdated';
	description: string;
	userId?: string;
	userEmail?: string;
	createdAt: Date;
}
