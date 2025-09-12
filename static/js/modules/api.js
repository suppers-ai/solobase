// API client module
export class APIClient {
    constructor() {
        this.baseURL = '/api';
    }

    async request(method, path, data = null) {
        const options = {
            method,
            headers: {
                'Content-Type': 'application/json',
            },
        };

        if (data) {
            options.body = JSON.stringify(data);
        }

        try {
            const response = await fetch(`${this.baseURL}${path}`, options);
            
            if (!response.ok) {
                const error = await response.json().catch(() => ({ error: response.statusText }));
                throw new Error(error.error || 'Request failed');
            }

            return await response.json();
        } catch (error) {
            console.error('API request failed:', error);
            throw error;
        }
    }

    // Collections
    collections = {
        list: () => this.request('GET', '/collections'),
        get: (name) => this.request('GET', `/collections/${name}`),
        create: (data) => this.request('POST', '/collections', data),
        update: (name, data) => this.request('PUT', `/collections/${name}`, data),
        delete: (name) => this.request('DELETE', `/collections/${name}`),
        
        // Records
        listRecords: (collection, params = {}) => {
            const query = new URLSearchParams(params).toString();
            return this.request('GET', `/collections/${collection}/records${query ? '?' + query : ''}`);
        },
        getRecord: (collection, id) => this.request('GET', `/collections/${collection}/records/${id}`),
        createRecord: (collection, data) => this.request('POST', `/collections/${collection}/records`, data),
        updateRecord: (collection, id, data) => this.request('PUT', `/collections/${collection}/records/${id}`, data),
        deleteRecord: (collection, id) => this.request('DELETE', `/collections/${collection}/records/${id}`),
    };

    // Auth
    auth = {
        login: (email, password) => this.request('POST', '/auth/login', { email, password }),
        signup: (data) => this.request('POST', '/auth/signup', data),
        logout: () => this.request('POST', '/auth/logout'),
        me: () => this.request('GET', '/auth/me'),
    };

    // Health
    health = () => this.request('GET', '/health');
}

// Export singleton instance
export const api = new APIClient();