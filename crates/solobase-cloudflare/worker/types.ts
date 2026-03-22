// Shared TypeScript types matching the WIT definitions from wafer-run/wit/wit/types.wit
// These mirror the jco-generated types for use in the CF Worker runtime.

// --- WIT core types ---

export interface MetaEntry {
  key: string;
  value: string;
}

export interface Message {
  kind: string;
  data: Uint8Array;
  meta: MetaEntry[];
}

export type Action = 'continue' | 'respond' | 'drop' | 'error';

export interface Response {
  data: Uint8Array;
  meta: MetaEntry[];
}

export type ErrorCode =
  | 'ok'
  | 'cancelled'
  | 'unknown'
  | 'invalid-argument'
  | 'deadline-exceeded'
  | 'not-found'
  | 'already-exists'
  | 'permission-denied'
  | 'resource-exhausted'
  | 'failed-precondition'
  | 'aborted'
  | 'out-of-range'
  | 'unimplemented'
  | 'internal'
  | 'unavailable'
  | 'data-loss'
  | 'unauthenticated';

export interface WaferError {
  code: ErrorCode;
  message: string;
  meta: MetaEntry[];
}

export interface BlockResult {
  action: Action;
  response?: Response;
  error?: WaferError;
  message?: Message;
}

export type InstanceMode = 'per-node' | 'singleton' | 'per-flow' | 'per-execution';

export interface BlockInfo {
  name: string;
  version: string;
  interface: string;
  summary: string;
  instanceMode: InstanceMode;
  allowedModes: InstanceMode[];
}

export type LifecycleType = 'init' | 'start' | 'stop';

export interface LifecycleEvent {
  eventType: LifecycleType;
  data: Uint8Array;
}

// --- Block interface (matches WIT block interface) ---

export interface Block {
  info(): BlockInfo;
  handle(msg: Message): BlockResult;
  lifecycle(event: LifecycleEvent): void;
}

// --- Cloudflare environment bindings ---

export interface Env {
  DB: D1Database;
  STORAGE: R2Bucket;
  TENANTS: KVNamespace;
  JWT_SECRET?: string;
  ADMIN_SECRET?: string;
  ENVIRONMENT?: string;
  STRIPE_SECRET_KEY?: string;
  STRIPE_WEBHOOK_SECRET?: string;
  STRIPE_PRICE_STARTER?: string;
  STRIPE_PRICE_PRO?: string;
  MAILGUN_API_KEY?: string;
  MAILGUN_DOMAIN?: string;
  MAILGUN_FROM?: string;
  [key: string]: unknown;
}

// --- Tenant types ---

export interface TenantConfig {
  id: string;
  subdomain: string;
  plan: string;
  owner_user_id?: string;
  db_id?: string;
  db_binding?: string;
  config: TenantAppConfig;
  blocks: string[];
}

// --- Plan limits ---

export interface PlanLimits {
  maxProjects: number;
  maxRequestsPerMonth: number;
  maxD1StorageBytes: number;
  maxR2StorageBytes: number;
  customDomain: boolean;
}

export const PLANS: Record<string, PlanLimits> = {
  starter: {
    maxProjects: 2,
    maxRequestsPerMonth: 500_000,
    maxD1StorageBytes: 500 * 1024 * 1024,       // 500 MB
    maxR2StorageBytes: 2 * 1024 * 1024 * 1024,   // 2 GB
    customDomain: false,
  },
  pro: {
    maxProjects: Infinity,
    maxRequestsPerMonth: 3_000_000,
    maxD1StorageBytes: 5 * 1024 * 1024 * 1024,   // 5 GB
    maxR2StorageBytes: 20 * 1024 * 1024 * 1024,   // 20 GB
    customDomain: true,
  },
  platform: {
    maxProjects: Infinity,
    maxRequestsPerMonth: Infinity,
    maxD1StorageBytes: Infinity,
    maxR2StorageBytes: Infinity,
    customDomain: true,
  },
};

export function getPlanLimits(plan: string): PlanLimits {
  return PLANS[plan] ?? PLANS['starter'];
}

export interface TenantAppConfig {
  version?: number;
  app?: string;
  auth?: unknown;
  admin?: unknown;
  files?: unknown;
  products?: unknown;
  deployments?: unknown;
  legalpages?: unknown;
  userportal?: unknown;
}

export type SubscriptionStatus = 'active' | 'past_due' | 'cancelled' | 'suspended';

export interface Subscription {
  id: string;
  user_id: string;
  plan: string;
  stripe_customer_id: string;
  stripe_subscription_id: string;
  status: SubscriptionStatus;
  current_period_end: string | null;
  grace_period_end: string | null;
}
