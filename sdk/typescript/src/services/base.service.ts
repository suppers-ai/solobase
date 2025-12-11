import axios, { AxiosInstance, AxiosRequestConfig, AxiosError } from 'axios';
import { SolobaseConfig, ApiResponse } from '../types';

export class BaseService {
  protected client: AxiosInstance;
  protected config: SolobaseConfig;

  constructor(config: SolobaseConfig) {
    this.config = config;
    this.client = axios.create({
      baseURL: config.url + '/api',
      timeout: config.timeout || 30000,
      withCredentials: true, // Send cookies with cross-origin requests
      headers: {
        'Content-Type': 'application/json',
        ...config.headers,
      },
    });

    // Add auth interceptor if API key is provided
    if (config.apiKey) {
      this.client.interceptors.request.use((reqConfig) => {
        reqConfig.headers.Authorization = `Bearer ${config.apiKey}`;
        return reqConfig;
      });
    }

    // Add response interceptor for error handling
    this.client.interceptors.response.use(
      (response) => response,
      (error: AxiosError) => {
        const apiError: ApiResponse = {
          success: false,
          error: {
            code: error.code || 'UNKNOWN_ERROR',
            message: error.message,
            details: error.response?.data,
          },
        };
        return Promise.reject(apiError);
      }
    );
  }

  protected async request<T>(config: AxiosRequestConfig): Promise<T> {
    try {
      const response = await this.client.request<ApiResponse<T>>(config);
      if (response.data.success === false) {
        throw response.data;
      }
      return response.data.data || (response.data as T);
    } catch (error) {
      throw error;
    }
  }

  protected buildQueryString(params: Record<string, any>): string {
    const query = new URLSearchParams();
    Object.entries(params).forEach(([key, value]) => {
      if (value !== undefined && value !== null) {
        if (typeof value === 'object') {
          query.append(key, JSON.stringify(value));
        } else {
          query.append(key, String(value));
        }
      }
    });
    return query.toString();
  }

  /**
   * Set API key for server-to-server authentication.
   * In browser environments, cookie-based auth is used automatically.
   */
  public setApiKey(apiKey: string) {
    this.config.apiKey = apiKey;
    this.client.defaults.headers.common['Authorization'] = `Bearer ${apiKey}`;
  }

  /**
   * Remove API key (for server-to-server auth).
   * In browser environments, use logout() to clear the auth cookie.
   */
  public removeApiKey() {
    delete this.config.apiKey;
    delete this.client.defaults.headers.common['Authorization'];
  }
}