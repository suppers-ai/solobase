/**
 * Static file serving from R2 with SPA fallback.
 *
 * Serves two kinds of sites:
 * - Platform SPA: files under `_site/` prefix in R2 (for cloud.solobase.dev)
 * - Project SPA: files under `{projectId}/site/` prefix in R2
 *
 * The marketing site (solobase.dev) is served by Cloudflare Pages, not here.
 *
 * SPA fallback: if no file is found, serves index.html for client-side routing.
 */

import type { Env, ProjectConfig } from './types';

/** MIME types by extension. */
const MIME_TYPES: Record<string, string> = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.mjs': 'application/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.webp': 'image/webp',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
  '.otf': 'font/otf',
  '.wasm': 'application/wasm',
  '.map': 'application/json',
  '.txt': 'text/plain; charset=utf-8',
  '.xml': 'application/xml',
  '.webmanifest': 'application/manifest+json',
};

function getMimeType(path: string): string {
  const ext = path.substring(path.lastIndexOf('.'));
  return MIME_TYPES[ext] ?? 'application/octet-stream';
}

/** Cache-Control: immutable for hashed assets, short for HTML. */
function getCacheControl(path: string): string {
  // Hashed assets (Vite pattern: name.abc123.js)
  if (/\.[a-f0-9]{8,}\.(js|css|woff2?|png|jpg|svg)$/.test(path)) {
    return 'public, max-age=31536000, immutable';
  }
  // HTML and other non-hashed files
  if (path.endsWith('.html') || !path.includes('.')) {
    return 'public, max-age=0, must-revalidate';
  }
  return 'public, max-age=3600';
}

/**
 * Serve a static file from R2.
 *
 * @param bucket R2 bucket
 * @param prefix R2 key prefix (e.g., "_site/" or "{projectId}/site/")
 * @param pathname URL path (e.g., "/", "/about", "/assets/app.js")
 * @returns Response or null if file not found (and SPA fallback also missing)
 */
export async function serveStatic(
  bucket: R2Bucket,
  prefix: string,
  pathname: string,
): Promise<Response | null> {
  // Normalize path
  let filePath = pathname === '/' ? '/index.html' : pathname;

  // Security: reject path traversal
  if (filePath.includes('..') || filePath.includes('\0')) {
    return new Response('Bad Request', { status: 400 });
  }

  // Strip leading slash for R2 key
  const key = prefix + filePath.replace(/^\//, '');

  // Try exact file
  let object = await bucket.get(key);

  // Trailing slash → try as directory index (e.g., /pricing/ → pricing/index.html)
  if (!object && filePath.endsWith('/')) {
    object = await bucket.get(key + 'index.html');
    if (object) filePath += 'index.html';
  }

  // If no extension, try with .html (e.g., /about → about.html)
  if (!object && !filePath.includes('.') && !filePath.endsWith('/')) {
    object = await bucket.get(key + '.html');
    if (object) filePath += '.html';
  }

  // If still not found, try as directory index without trailing slash
  if (!object && !filePath.includes('.') && !filePath.endsWith('/')) {
    object = await bucket.get(key + '/index.html');
    if (object) filePath += '/index.html';
  }

  if (object) {
    return buildFileResponse(object, filePath);
  }

  // SPA fallback: serve index.html ONLY for known app routes.
  // Marketing routes (/docs, /pricing, /about) should NOT fallback —
  // those belong on CF Pages (solobase.dev), not the Worker (cloud.solobase.dev).
  if (isSpaRoute(pathname)) {
    const indexKey = prefix + 'index.html';
    const indexObject = await bucket.get(indexKey);
    if (indexObject) {
      return buildFileResponse(indexObject, '/index.html');
    }
  }

  // No site deployed
  return null;
}

/**
 * Only these routes should get SPA fallback (index.html).
 * Everything else returns 404 — marketing routes belong on CF Pages.
 */
function isSpaRoute(pathname: string): boolean {
  const spaRoutes = [
    '/blocks/', '/admin', '/auth', '/dashboard',
    '/settings', '/login', '/signup',
  ];
  return pathname === '/' || spaRoutes.some(r => pathname.startsWith(r));
}

function buildFileResponse(object: R2ObjectBody, filePath: string): Response {
  const headers = new Headers();
  headers.set('Content-Type', getMimeType(filePath));
  headers.set('Cache-Control', getCacheControl(filePath));

  // ETag for conditional requests
  if (object.etag) {
    headers.set('ETag', object.etag);
  }

  return new Response(object.body, { headers });
}

/**
 * Determine the R2 prefix for a request.
 *
 * - Platform (cloud.solobase.dev): `_site/`
 * - Project site: `{projectId}/site/`
 */
export function getR2Prefix(project: ProjectConfig | null, isMarketingSite: boolean): string {
  if (isMarketingSite) return '_site/';
  return project ? `${project.id}/site/` : '_site/';
}

/**
 * Check if this is the platform host (cloud.solobase.dev or localhost in dev).
 *
 * The marketing site (solobase.dev) is now served by Cloudflare Pages, not
 * this Worker. The Worker only handles:
 * - cloud.solobase.dev → platform API + SPA (dashboard/admin/auth from R2)
 * - {project}.solobase.dev → project API + SPA
 */
export function isMarketingHost(hostname: string): boolean {
  const hostNoPort = hostname.split(':')[0];
  const parts = hostNoPort.split('.');

  // localhost → platform (dev mode)
  if (hostNoPort === 'localhost' || hostNoPort.startsWith('127.')) return true;

  // cloud.solobase.dev → platform
  if (parts[0] === 'cloud') return true;

  return false;
}
