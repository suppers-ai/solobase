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

export interface User {
	email: string;
	role?: string;
	id?: string;
	name?: string;
}