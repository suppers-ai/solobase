// Shared auth types
import type { UserResponse } from './models';

export interface LoginRequest {
	email: string;
	password: string;
	remember?: boolean;
}

export interface LoginResponse {
	data: UserResponse;
	message?: string;
}

export interface SignupRequest {
	email: string;
	password: string;
	username?: string;
	first_name?: string;
	last_name?: string;
}

export interface LoginCredentials {
	email: string;
	password: string;
	remember?: boolean;
}

export interface RegisterCredentials {
	email: string;
	password: string;
	confirm_password?: string;
	username?: string;
	first_name?: string;
	last_name?: string;
	terms_accepted?: boolean;
}

export interface OAuthProvider {
	id: string;
	name: string;
	enabled: boolean;
	client_id?: string;
	icon?: string;
	color?: string;
}

export interface PasswordResetRequest {
	email: string;
}

export interface PasswordReset {
	token: string;
	new_password: string;
	confirm_password: string;
}

export interface EmailVerification {
	token: string;
	email?: string;
}

export interface TwoFactorAuth {
	enabled: boolean;
	method: 'totp' | 'sms' | 'email';
	verified: boolean;
	backup_codes?: string[];
}

export interface Permission {
	id: string;
	name: string;
	description?: string;
	resource?: string;
	action?: string;
	created_at: string;
}
