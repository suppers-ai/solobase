import { SolobaseConfig } from './types';
import { AuthService } from './services/auth.service';
import { StorageService } from './services/storage.service';
import { DatabaseService } from './services/database.service';
import { ExtensionsService, CloudStorageExtension, ProductsExtension } from './services/extensions.service';

export class SolobaseClient {
  public auth: AuthService;
  public storage: StorageService;
  public database: DatabaseService;
  public extensions: ExtensionsService;
  
  // Extension-specific services
  public cloudStorage: CloudStorageExtension;
  public products: ProductsExtension;

  private config: SolobaseConfig;

  constructor(config: SolobaseConfig | string) {
    // If string is passed, treat it as URL
    if (typeof config === 'string') {
      this.config = { url: config };
    } else {
      this.config = config;
    }

    // Ensure URL doesn't have trailing slash
    this.config.url = this.config.url.replace(/\/$/, '');

    // Initialize services
    this.auth = new AuthService(this.config);
    this.storage = new StorageService(this.config);
    this.database = new DatabaseService(this.config);
    this.extensions = new ExtensionsService(this.config);
    
    // Initialize extension-specific services
    this.cloudStorage = new CloudStorageExtension(this.config);
    this.products = new ProductsExtension(this.config);

    // Bind auth state changes to all services
    this.setupAuthSync();
  }

  /**
   * Setup auth token synchronization across all services
   */
  private setupAuthSync() {
    // Override auth service methods to sync tokens
    const originalSetTokens = this.auth.setTokens.bind(this.auth);
    this.auth.setTokens = (tokens) => {
      originalSetTokens(tokens);
      this.syncAuthToken(tokens.access_token);
    };

    const originalSignIn = this.auth.signIn.bind(this.auth);
    this.auth.signIn = async (options) => {
      const result = await originalSignIn(options);
      this.syncAuthToken(result.tokens.access_token);
      return result;
    };

    const originalSignUp = this.auth.signUp.bind(this.auth);
    this.auth.signUp = async (options) => {
      const result = await originalSignUp(options);
      this.syncAuthToken(result.tokens.access_token);
      return result;
    };

    const originalSignOut = this.auth.signOut.bind(this.auth);
    this.auth.signOut = async () => {
      await originalSignOut();
      this.syncAuthToken(null);
    };

    const originalRefreshToken = this.auth.refreshToken.bind(this.auth);
    this.auth.refreshToken = async () => {
      const tokens = await originalRefreshToken();
      this.syncAuthToken(tokens.access_token);
      return tokens;
    };
  }

  /**
   * Sync auth token across all services
   */
  private syncAuthToken(token: string | null) {
    const services = [
      this.storage,
      this.database,
      this.extensions,
      this.cloudStorage,
      this.products,
    ];

    services.forEach(service => {
      if (token) {
        service.setAuthToken(token);
      } else {
        service.removeAuthToken();
      }
    });
  }

  /**
   * Set a global API key for all services
   */
  public setApiKey(apiKey: string) {
    this.config.apiKey = apiKey;
    this.syncAuthToken(apiKey);
  }

  /**
   * Remove the global API key
   */
  public removeApiKey() {
    delete this.config.apiKey;
    this.syncAuthToken(null);
  }

  /**
   * Get the current configuration
   */
  public getConfig(): SolobaseConfig {
    return { ...this.config };
  }

  /**
   * Check if client is authenticated
   */
  public isAuthenticated(): boolean {
    return this.auth.isAuthenticated();
  }

}

// Export a factory function for convenience
export function createSolobaseClient(config: SolobaseConfig | string): SolobaseClient {
  return new SolobaseClient(config);
}