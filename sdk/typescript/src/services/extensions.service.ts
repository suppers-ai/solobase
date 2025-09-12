import { BaseService } from './base.service';

export interface Extension {
  name: string;
  version: string;
  description: string;
  author: string;
  enabled: boolean;
  config?: Record<string, any>;
  metadata?: {
    tags?: string[];
    homepage?: string;
    license?: string;
  };
}

export interface ExtensionHealth {
  extension: string;
  status: 'healthy' | 'unhealthy' | 'unknown';
  message: string;
  last_checked: string;
}

export class ExtensionsService extends BaseService {
  /**
   * List all available extensions
   */
  async list(): Promise<Extension[]> {
    return this.request<Extension[]>({
      method: 'GET',
      url: '/extensions',
    });
  }

  /**
   * Get details of a specific extension
   */
  async get(name: string): Promise<Extension> {
    return this.request<Extension>({
      method: 'GET',
      url: `/extensions/${name}`,
    });
  }

  /**
   * Enable an extension
   */
  async enable(name: string, config?: Record<string, any>): Promise<void> {
    await this.request<void>({
      method: 'POST',
      url: `/extensions/${name}/enable`,
      data: config,
    });
  }

  /**
   * Disable an extension
   */
  async disable(name: string): Promise<void> {
    await this.request<void>({
      method: 'POST',
      url: `/extensions/${name}/disable`,
    });
  }

  /**
   * Update extension configuration
   */
  async updateConfig(name: string, config: Record<string, any>): Promise<void> {
    await this.request<void>({
      method: 'PATCH',
      url: `/extensions/${name}/config`,
      data: config,
    });
  }

  /**
   * Get extension health status
   */
  async health(name?: string): Promise<ExtensionHealth | ExtensionHealth[]> {
    const url = name ? `/extensions/${name}/health` : '/extensions/health';
    return this.request<ExtensionHealth | ExtensionHealth[]>({
      method: 'GET',
      url,
    });
  }

  /**
   * Call a custom extension endpoint
   */
  async call<T = any>(
    extension: string,
    endpoint: string,
    options?: {
      method?: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';
      data?: any;
      params?: Record<string, any>;
    }
  ): Promise<T> {
    const queryString = options?.params ? this.buildQueryString(options.params) : '';
    return this.request<T>({
      method: options?.method || 'GET',
      url: `/extensions/${extension}/${endpoint}${queryString ? `?${queryString}` : ''}`,
      data: options?.data,
    });
  }
}

// CloudStorage extension specific methods
export class CloudStorageExtension extends ExtensionsService {
  /**
   * Share a file or folder
   */
  async share(objectId: string, options: {
    email?: string;
    userId?: string;
    permissions?: 'view' | 'edit' | 'admin';
    expiresAt?: string;
    isPublic?: boolean;
  }): Promise<{ share_id: string; share_token?: string; url?: string }> {
    return this.call('cloudstorage', 'api/shares', {
      method: 'POST',
      data: {
        object_id: objectId,
        ...options,
      },
    });
  }

  /**
   * List shares for current user
   */
  async listShares(): Promise<Array<{
    id: string;
    object_id: string;
    shared_with_email?: string;
    shared_with_user_id?: string;
    permission_level: string;
    is_public: boolean;
    expires_at?: string;
    share_token?: string;
  }>> {
    return this.call('cloudstorage', 'api/shares');
  }

  /**
   * Delete a share
   */
  async deleteShare(shareId: string): Promise<void> {
    await this.call('cloudstorage', `api/shares/${shareId}`, {
      method: 'DELETE',
    });
  }

  /**
   * Get storage quota information
   */
  async getQuota(): Promise<{
    max_storage_bytes: number;
    max_bandwidth_bytes: number;
    storage_used: number;
    bandwidth_used: number;
    reset_bandwidth_at?: string;
  }> {
    return this.call('cloudstorage', 'api/quota');
  }

  /**
   * Get access logs
   */
  async getAccessLogs(objectId?: string): Promise<Array<{
    id: string;
    object_id: string;
    user_id?: string;
    action: string;
    ip_address?: string;
    user_agent?: string;
    created_at: string;
  }>> {
    return this.call('cloudstorage', 'api/access-logs', {
      params: objectId ? { object_id: objectId } : undefined,
    });
  }

  /**
   * Get access statistics
   */
  async getAccessStats(): Promise<{
    total_views: number;
    total_downloads: number;
    unique_visitors: number;
    top_files: Array<{ object_id: string; count: number }>;
    daily_stats: Array<{ date: string; views: number; downloads: number }>;
  }> {
    return this.call('cloudstorage', 'api/access-stats');
  }
}

// Products extension specific methods
export class ProductsExtension extends ExtensionsService {
  /**
   * List products
   */
  async listProducts(groupId?: string): Promise<any[]> {
    return this.call('products', 'api/products', {
      params: groupId ? { group_id: groupId } : undefined,
    });
  }

  /**
   * Create a product
   */
  async createProduct(data: {
    group_id: string;
    name: string;
    template_id?: string;
    custom_fields?: Record<string, any>;
    pricing_formula?: string;
  }): Promise<any> {
    return this.call('products', 'api/products', {
      method: 'POST',
      data,
    });
  }

  /**
   * Calculate price for a product
   */
  async calculatePrice(productId: string, variables: Record<string, any>): Promise<{
    price: number;
    breakdown: Array<{ component: string; value: number }>;
  }> {
    return this.call('products', `api/products/${productId}/calculate`, {
      method: 'POST',
      data: { variables },
    });
  }

  /**
   * List groups
   */
  async listGroups(): Promise<any[]> {
    return this.call('products', 'api/groups');
  }

  /**
   * Create a group
   */
  async createGroup(data: {
    name: string;
    template_id: string;
    custom_fields?: Record<string, any>;
  }): Promise<any> {
    return this.call('products', 'api/groups', {
      method: 'POST',
      data,
    });
  }
}