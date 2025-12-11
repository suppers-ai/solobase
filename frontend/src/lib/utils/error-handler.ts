import { toasts } from '$lib/stores/toast';
import type { ApiError } from '$lib/types/api.types';

export class ErrorHandler {
	private static readonly ERROR_MESSAGES: Record<string, string> = {
		// Auth errors
		'AUTH_INVALID_CREDENTIALS': 'Invalid email or password',
		'AUTH_USER_NOT_FOUND': 'User not found',
		'AUTH_EMAIL_NOT_VERIFIED': 'Please verify your email address',
		'AUTH_ACCOUNT_DISABLED': 'Your account has been disabled',
		'AUTH_SESSION_EXPIRED': 'Your session has expired. Please login again',
		'AUTH_INSUFFICIENT_PERMISSIONS': 'You do not have permission to perform this action',
		
		// Validation errors
		'VALIDATION_REQUIRED_FIELD': 'This field is required',
		'VALIDATION_INVALID_EMAIL': 'Please enter a valid email address',
		'VALIDATION_PASSWORD_TOO_SHORT': 'Password must be at least 8 characters',
		'VALIDATION_PASSWORDS_DO_NOT_MATCH': 'Passwords do not match',
		
		// Network errors
		'NETWORK_ERROR': 'Network error. Please check your connection',
		'TIMEOUT_ERROR': 'Request timed out. Please try again',
		'SERVER_ERROR': 'Server error. Please try again later',
		
		// Storage errors
		'STORAGE_QUOTA_EXCEEDED': 'Storage quota exceeded',
		'STORAGE_FILE_TOO_LARGE': 'File size exceeds the maximum allowed',
		'STORAGE_INVALID_FILE_TYPE': 'Invalid file type',
		'STORAGE_BUCKET_NOT_FOUND': 'Storage bucket not found',
		
		// Database errors
		'DATABASE_CONNECTION_ERROR': 'Database connection failed',
		'DATABASE_QUERY_ERROR': 'Database query failed',
		'DATABASE_CONSTRAINT_ERROR': 'Database constraint violation',
		
		// Generic errors
		'NOT_FOUND': 'Resource not found',
		'CONFLICT': 'Resource already exists',
		'BAD_REQUEST': 'Invalid request',
		'FORBIDDEN': 'Access forbidden',
		'UNAUTHORIZED': 'Authentication required'
	};

	/**
	 * Handle API errors and show appropriate user feedback
	 */
	static handle(error: any, showToast = true): string {
		let message = 'An unexpected error occurred';
		let title = 'Error';

		// Handle different error types
		if (error instanceof Error) {
			message = error.message;
		} else if (typeof error === 'string') {
			message = error;
		} else if (error?.response) {
			// Axios-like error response
			const { data, status } = error.response;
			
			if (data?.error) {
				message = this.getErrorMessage(data.error);
			} else if (data?.message) {
				message = data.message;
			}

			title = this.getErrorTitle(status);
		} else if (error?.error) {
			// API error format
			const apiError = error.error as ApiError;
			message = this.getErrorMessage(apiError.code) || apiError.message;
			
			if (apiError.field) {
				message = `${apiError.field}: ${message}`;
			}
		}

		// Check for network errors
		if (!navigator.onLine) {
			message = 'No internet connection';
			title = 'Offline';
		}

		// Show toast notification
		if (showToast) {
			toasts.error(message, title);
		}

		// Log error for debugging
		console.error('[ErrorHandler]', error);

		return message;
	}

	/**
	 * Get user-friendly error message from error code
	 */
	private static getErrorMessage(code: string): string {
		return this.ERROR_MESSAGES[code] || code.replace(/_/g, ' ').toLowerCase();
	}

	/**
	 * Get error title based on HTTP status code
	 */
	private static getErrorTitle(status: number): string {
		switch (status) {
			case 400: return 'Bad Request';
			case 401: return 'Authentication Error';
			case 403: return 'Access Denied';
			case 404: return 'Not Found';
			case 409: return 'Conflict';
			case 422: return 'Validation Error';
			case 429: return 'Too Many Requests';
			case 500: return 'Server Error';
			case 503: return 'Service Unavailable';
			default: return 'Error';
		}
	}

	/**
	 * Handle form validation errors
	 */
	static handleValidation(errors: Record<string, string[]>): void {
		const errorMessages = Object.entries(errors)
			.map(([field, messages]) => `${field}: ${messages.join(', ')}`)
			.join('\n');

		toasts.error(errorMessages, 'Validation Error');
	}

	/**
	 * Create a retry mechanism for failed requests
	 */
	static async retry<T>(
		fn: () => Promise<T>,
		retries = 3,
		delay = 1000
	): Promise<T> {
		try {
			return await fn();
		} catch (error) {
			if (retries > 0) {
				await new Promise(resolve => setTimeout(resolve, delay));
				return this.retry(fn, retries - 1, delay * 2);
			}
			throw error;
		}
	}
}