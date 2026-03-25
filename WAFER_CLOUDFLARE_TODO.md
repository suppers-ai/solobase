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
- [x] Secrets forwarded to user workers during provisioning
- [x] `provision-cloud.sh` updated to use namespace name

## Remaining

- [ ] Configure Stripe test mode keys on dev "cloud" user worker
- [ ] Clean up old `dev.solobase.dev` custom domain and DNS records from solobase.dev zone
- [ ] Clean up old `DISPATCHER_NAMESPACE_ID` secret from dev dispatch worker
- [ ] Test full CI pipeline end-to-end
- [ ] Add health check after provisioning to verify user worker is responsive
