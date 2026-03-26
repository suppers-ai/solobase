# Workers for Platforms — Remaining TODO

## Done

- [x] Split solobase-cloudflare into dispatch worker + solobase-worker user worker
- [x] Per-project D1 isolation via WfP dispatch namespace
- [x] Dev environment: `cloud.solobase-dev.dev` + `{project}.solobase-dev.dev`
- [x] Production environment: `cloud.solobase.dev` + `{project}.solobase.dev`
- [x] Frontend assets uploaded to both environments
- [x] CI/CD workflow updated (manual deploy for now)
- [x] `AUTH_ALLOWED_EMAIL_DOMAINS` feature for restricting signups
- [x] Rollback on failed provisioning (delete D1 if worker upload fails)
- [x] Variables table — all project config stored in D1 (portable)
- [x] JWT_SECRET auto-generated per project, stored in variables table
- [x] No secrets forwarded during provisioning — each project is self-contained
- [x] Rename ADMIN_SECRET to CONTROL_API_KEY
- [x] Cleaned up unused secrets from dispatch worker
- [x] Fixed redirect loop (BlockShell/FeatureShell/Sidebar)
- [x] Fixed HTML caching (no-cache for HTML, immutable for hashed assets)
- [x] Fixed SPA fallback (block-specific index.html, not root)
- [x] 404 page with "Go to Dashboard" button
- [x] safeRedirect utility with loop detection
- [x] Admin button + sidebar links updated to /blocks/admin/frontend/
- [x] ADMIN_EMAIL + CONTROL_PLANE_URL + CONTROL_PLANE_SECRET set on both environments
- [x] Fixed npm build overwriting index.html (source template updated)
- [x] Cleaned up old dev.solobase.dev DNS records
- [x] Tested project creation end-to-end (D1 isolation verified)

## Remaining

- [ ] Test full CI pipeline end-to-end (push to main)
- [ ] Add health check after provisioning to verify user worker is responsive
- [ ] Commit all changes
