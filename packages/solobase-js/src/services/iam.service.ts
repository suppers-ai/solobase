import { BaseService } from "./base.service";
import { IAMRole, IAMPolicy, IAMAuditLog, User } from "../types";

export interface UserWithRoles extends User {
  roles: string[];
}

export class IAMService extends BaseService {
  /**
   * Get all roles
   */
  async getRoles(): Promise<IAMRole[]> {
    return this.request<IAMRole[]>({
      method: "GET",
      url: "/b/admin/api/iam/roles",
    });
  }

  /**
   * Create a new role
   */
  async createRole(role: IAMRole): Promise<IAMRole> {
    return this.request<IAMRole>({
      method: "POST",
      url: "/b/admin/api/iam/roles",
      data: role,
    });
  }

  /**
   * Update a role
   */
  async updateRole(
    roleName: string,
    updates: Partial<IAMRole>,
  ): Promise<IAMRole> {
    return this.request<IAMRole>({
      method: "PUT",
      url: `/b/admin/api/iam/roles/${roleName}`,
      data: updates,
    });
  }

  /**
   * Delete a role
   */
  async deleteRole(roleName: string): Promise<void> {
    await this.request<void>({
      method: "DELETE",
      url: `/b/admin/api/iam/roles/${roleName}`,
    });
  }

  /**
   * Get all policies
   */
  async getPolicies(): Promise<IAMPolicy[]> {
    return this.request<IAMPolicy[]>({
      method: "GET",
      url: "/b/admin/api/iam/policies",
    });
  }

  /**
   * Create a new policy
   */
  async createPolicy(policy: IAMPolicy): Promise<void> {
    await this.request<void>({
      method: "POST",
      url: "/b/admin/api/iam/policies",
      data: policy,
    });
  }

  /**
   * Delete a policy
   */
  async deletePolicy(policyId: string): Promise<void> {
    await this.request<void>({
      method: "DELETE",
      url: `/b/admin/api/iam/policies/${policyId}`,
    });
  }

  /**
   * Get all users with their roles
   */
  async getUsersWithRoles(): Promise<UserWithRoles[]> {
    return this.request<UserWithRoles[]>({
      method: "GET",
      url: "/b/admin/api/iam/users",
    });
  }

  /**
   * Assign a role to a user
   */
  async assignRoleToUser(userId: string, roleName: string): Promise<void> {
    await this.request<void>({
      method: "POST",
      url: `/b/admin/api/iam/users/${userId}/roles`,
      data: { role: roleName },
    });
  }

  /**
   * Remove a role from a user
   */
  async removeRoleFromUser(userId: string, roleName: string): Promise<void> {
    await this.request<void>({
      method: "DELETE",
      url: `/b/admin/api/iam/users/${userId}/roles/${roleName}`,
    });
  }

  /**
   * Test a permission
   */
  async testPermission(
    userId: string,
    resource: string,
    action: string,
  ): Promise<{
    allowed: boolean;
    userRoles: string[];
  }> {
    return this.request<{ allowed: boolean; userRoles: string[] }>({
      method: "POST",
      url: "/b/admin/api/iam/test-permission",
      data: {
        userId,
        resource,
        action,
      },
    });
  }

  /**
   * Get audit logs
   */
  async getAuditLogs(options?: {
    limit?: number;
    filter?: string;
    type?: string;
  }): Promise<IAMAuditLog[]> {
    const params = new URLSearchParams();
    if (options?.limit) params.append("limit", options.limit.toString());
    if (options?.filter) params.append("filter", options.filter);
    if (options?.type) params.append("type", options.type);

    const queryString = params.toString();
    const url = queryString
      ? `/b/admin/api/iam/audit-logs?${queryString}`
      : "/b/admin/api/iam/audit-logs";

    return this.request<IAMAuditLog[]>({
      method: "GET",
      url,
    });
  }

  /**
   * Get user's roles
   */
  async getUserRoles(userId: string): Promise<string[]> {
    const response = await this.request<{ roles: string[] }>({
      method: "GET",
      url: `/b/admin/api/iam/users/${userId}/roles`,
    });
    return response.roles;
  }

  /**
   * Check if user has a specific role
   */
  async userHasRole(userId: string, roleName: string): Promise<boolean> {
    const roles = await this.getUserRoles(userId);
    return roles.includes(roleName);
  }

  /**
   * Check if user has any of the specified roles
   */
  async userHasAnyRole(userId: string, roleNames: string[]): Promise<boolean> {
    const roles = await this.getUserRoles(userId);
    return roleNames.some((role) => roles.includes(role));
  }

  /**
   * Check if user has all of the specified roles
   */
  async userHasAllRoles(userId: string, roleNames: string[]): Promise<boolean> {
    const roles = await this.getUserRoles(userId);
    return roleNames.every((role) => roles.includes(role));
  }
}
