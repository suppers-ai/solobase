# Workers for Platforms — Remaining TODO

## Development Environment

- [x] Set up dev domain: `solobase-dev.dev` with wildcard DNS + custom domain
- [x] Platform at `cloud.solobase-dev.dev`, projects at `{project}.solobase-dev.dev`
- [x] Deploy dispatch worker with WfP dispatch
- [x] Build and upload frontend assets to dev R2
- [x] Run migrations on "cloud" user worker
- [x] Restrict dev signups via `AUTH_ALLOWED_EMAIL_DOMAINS=suppers.ai`
- [ ] Configure Stripe test mode keys on dev "cloud" user worker
- [ ] Clean up old `dev.solobase.dev` custom domain and DNS records from solobase.dev zone

## Production Environment

- [x] Set secrets on production dispatch worker
- [x] Deploy dispatch worker to production
- [x] Upload user worker artifacts to production R2
- [x] Provision "cloud" user worker in production namespace
- [x] Run platform + user worker migrations
- [x] Set up `cloud.solobase.dev` custom domain + `*.solobase.dev` route
- [x] Upload frontend assets to production R2
- [x] Verify `cloud.solobase.dev` serves dashboard + API

## CI/CD

- [x] Add `ADMIN_SECRET` to GitHub Actions secrets
- [x] Add `PLATFORM_URL` as a GitHub Actions variable
- [x] Fix CI workflow (index.js instead of shim.mjs for R2 upload)
- [ ] Test full CI pipeline: push to main → builds both workers → deploys → updates all user workers

## Code Improvements

- [x] Add rollback in `provision.rs` — delete D1 if worker upload fails
- [x] Forward secrets (JWT_SECRET, MAILGUN_*, STRIPE_*) to user workers during provisioning
- [ ] Update `provision-cloud.sh` to use namespace name instead of ID
- [ ] Consider adding health check after provisioning to verify user worker is responsive
