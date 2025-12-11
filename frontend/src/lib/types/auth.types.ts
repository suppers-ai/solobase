// Re-export shared auth types
export type {
	AuthUser,
	AuthToken,
	Role,
	UserResponse,
	LoginCredentials,
	RegisterCredentials,
	OAuthProvider,
	PasswordResetRequest,
	PasswordReset,
	EmailVerification,
	TwoFactorAuth,
	Permission,
	LoginRequest,
	LoginResponse,
	SignupRequest
} from '$shared/types';

// Frontend-specific auth types
import type { AuthUser } from '$shared/types';

export interface AuthState {
	user: AuthUser | null;
	isAuthenticated: boolean;
	isLoading: boolean;
	error: string | null;
}
