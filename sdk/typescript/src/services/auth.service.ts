import { BaseService } from './base.service';
import { User, AuthTokens } from '../types';

export interface SignUpOptions {
  email: string;
  password: string;
  metadata?: Record<string, any>;
}

export interface SignInOptions {
  email: string;
  password: string;
}

export interface ResetPasswordOptions {
  email: string;
}

export interface UpdatePasswordOptions {
  currentPassword: string;
  newPassword: string;
}

export class AuthService extends BaseService {
  private currentUser: User | null = null;
  private tokens: AuthTokens | null = null;

  /**
   * Sign up a new user
   */
  async signUp(options: SignUpOptions): Promise<{ user: User; tokens: AuthTokens }> {
    // Signup returns just the user, need to login after
    const user = await this.request<User>({
      method: 'POST',
      url: '/auth/signup',
      data: options,
    });
    
    // Now sign in to get the token
    const loginResponse = await this.request<{ token: string; user: User }>({
      method: 'POST',
      url: '/auth/login',
      data: {
        email: options.email,
        password: options.password,
      },
    });
    
    const tokens: AuthTokens = {
      access_token: loginResponse.token,
      expires_in: 86400, // 24 hours default
      token_type: 'Bearer',
    };
    
    this.currentUser = loginResponse.user;
    this.tokens = tokens;
    this.setAuthToken(tokens.access_token);
    
    return { user: loginResponse.user, tokens };
  }

  /**
   * Sign in an existing user
   */
  async signIn(options: SignInOptions): Promise<{ user: User; tokens: AuthTokens }> {
    const response = await this.request<{ token: string; user: User }>({
      method: 'POST',
      url: '/auth/login',
      data: options,
    });
    
    const tokens: AuthTokens = {
      access_token: response.token,
      expires_in: 86400, // 24 hours default
      token_type: 'Bearer',
    };
    
    this.currentUser = response.user;
    this.tokens = tokens;
    this.setAuthToken(tokens.access_token);
    
    return { user: response.user, tokens };
  }

  /**
   * Sign out the current user
   */
  async signOut(): Promise<void> {
    try {
      await this.request<void>({
        method: 'POST',
        url: '/auth/logout',
      });
    } finally {
      this.currentUser = null;
      this.tokens = null;
      this.removeAuthToken();
    }
  }

  /**
   * Get the current authenticated user
   */
  async getUser(): Promise<User | null> {
    if (!this.tokens?.access_token) {
      return null;
    }

    try {
      const user = await this.request<User>({
        method: 'GET',
        url: '/auth/me',
      });
      this.currentUser = user;
      return user;
    } catch (error) {
      return null;
    }
  }

  /**
   * Update the current user's profile
   */
  async updateUser(updates: Partial<User>): Promise<User> {
    const user = await this.request<User>({
      method: 'PATCH',
      url: '/auth/me',
      data: updates,
    });
    this.currentUser = user;
    return user;
  }

  /**
   * Request a password reset
   */
  async resetPassword(options: ResetPasswordOptions): Promise<void> {
    await this.request<void>({
      method: 'POST',
      url: '/auth/reset-password',
      data: options,
    });
  }

  /**
   * Confirm password reset with token
   */
  async confirmPasswordReset(token: string, newPassword: string): Promise<void> {
    await this.request<void>({
      method: 'POST',
      url: '/auth/confirm-reset',
      data: {
        token,
        password: newPassword,
      },
    });
  }

  /**
   * Update password for authenticated user
   */
  async updatePassword(options: UpdatePasswordOptions): Promise<void> {
    await this.request<void>({
      method: 'POST',
      url: '/auth/change-password',
      data: options,
    });
  }

  /**
   * Refresh the access token
   */
  async refreshToken(): Promise<AuthTokens> {
    if (!this.tokens?.refresh_token) {
      throw new Error('No refresh token available');
    }

    const tokens = await this.request<AuthTokens>({
      method: 'POST',
      url: '/auth/refresh',
      data: {
        refresh_token: this.tokens.refresh_token,
      },
    });
    
    this.tokens = tokens;
    this.setAuthToken(tokens.access_token);
    
    return tokens;
  }

  /**
   * Verify email with token
   */
  async verifyEmail(token: string): Promise<void> {
    await this.request<void>({
      method: 'POST',
      url: '/auth/verify-email',
      data: { token },
    });
  }

  /**
   * Resend verification email
   */
  async resendVerification(): Promise<void> {
    await this.request<void>({
      method: 'POST',
      url: '/auth/resend-verification',
    });
  }

  /**
   * Get the current user from memory (without API call)
   */
  getCurrentUser(): User | null {
    return this.currentUser;
  }

  /**
   * Get the current auth tokens from memory
   */
  getTokens(): AuthTokens | null {
    return this.tokens;
  }

  /**
   * Set auth tokens (useful for restoring session)
   */
  setTokens(tokens: AuthTokens): void {
    this.tokens = tokens;
    this.setAuthToken(tokens.access_token);
  }

  /**
   * Check if user is authenticated
   */
  isAuthenticated(): boolean {
    return !!this.tokens?.access_token;
  }

  /**
   * OAuth sign in
   */
  async signInWithOAuth(provider: 'google' | 'github' | 'facebook'): Promise<{ url: string }> {
    return this.request<{ url: string }>({
      method: 'GET',
      url: `/auth/oauth/${provider}`,
    });
  }

  /**
   * Handle OAuth callback
   */
  async handleOAuthCallback(
    provider: string,
    code: string
  ): Promise<{ user: User; tokens: AuthTokens }> {
    const response = await this.request<{ user: User; tokens: AuthTokens }>({
      method: 'POST',
      url: `/auth/oauth/${provider}/callback`,
      data: { code },
    });
    
    this.currentUser = response.user;
    this.tokens = response.tokens;
    this.setAuthToken(response.tokens.access_token);
    
    return response;
  }

  /**
   * Delete account
   */
  async deleteAccount(): Promise<void> {
    await this.request<void>({
      method: 'DELETE',
      url: '/auth/account',
    });
    
    this.currentUser = null;
    this.tokens = null;
    this.removeAuthToken();
  }
}