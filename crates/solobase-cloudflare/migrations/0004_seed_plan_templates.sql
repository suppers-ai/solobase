-- Seed SaaS plan templates and default products

-- Group template for SaaS plans
INSERT OR IGNORE INTO block_products_group_templates (id, name, display_name)
VALUES ('tpl-saas-plans', 'saas-plans', 'SaaS Plans');

-- Product template for individual SaaS plans
INSERT OR IGNORE INTO block_products_product_templates (id, name, display_name)
VALUES ('tpl-saas-plan', 'saas-plan', 'SaaS Plan');

-- Default product type: Subscription
INSERT OR IGNORE INTO block_products_types (id, name, description, is_system)
VALUES ('type-subscription', 'Subscription', 'Recurring subscription plan', 1);

-- Plans group
INSERT OR IGNORE INTO block_products_groups (id, name, description, group_template_id, status)
VALUES ('group-plans', 'Plans', 'Solobase subscription plans', 'tpl-saas-plans', 'active');

-- Starter plan
INSERT OR IGNORE INTO block_products_products (id, name, description, slug, price, currency, status, group_id, type_id, product_template_id, metadata)
VALUES (
  'prod-starter',
  'Starter',
  'For side projects and small apps',
  'starter',
  5,
  'USD',
  'published',
  'group-plans',
  'type-subscription',
  'tpl-saas-plan',
  '{"max_projects":"2","max_requests":"500000","d1_storage":"500MB","r2_storage":"2GB","custom_domain":"false","support":"Community support"}'
);

-- Pro plan
INSERT OR IGNORE INTO block_products_products (id, name, description, slug, price, currency, status, group_id, type_id, product_template_id, metadata)
VALUES (
  'prod-pro',
  'Pro',
  'For growing apps and production workloads',
  'pro',
  25,
  'USD',
  'published',
  'group-plans',
  'type-subscription',
  'tpl-saas-plan',
  '{"max_projects":"unlimited","max_requests":"3000000","d1_storage":"5GB","r2_storage":"20GB","custom_domain":"true","support":"Priority email support","daily_backups":"true"}'
);
