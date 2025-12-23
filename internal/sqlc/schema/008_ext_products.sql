-- Products extension tables

CREATE TABLE IF NOT EXISTS ext_products_variables (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT,
    value_type TEXT,
    type TEXT,
    default_value TEXT,
    description TEXT,
    status TEXT DEFAULT 'active',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ext_products_group_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT,
    description TEXT,
    icon TEXT,
    filter_fields_schema TEXT,
    status TEXT DEFAULT 'active',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ext_products_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    group_template_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    -- Filter columns
    filter_numeric_1 REAL,
    filter_numeric_2 REAL,
    filter_numeric_3 REAL,
    filter_numeric_4 REAL,
    filter_numeric_5 REAL,
    filter_text_1 TEXT,
    filter_text_2 TEXT,
    filter_text_3 TEXT,
    filter_text_4 TEXT,
    filter_text_5 TEXT,
    filter_boolean_1 INTEGER,
    filter_boolean_2 INTEGER,
    filter_boolean_3 INTEGER,
    filter_boolean_4 INTEGER,
    filter_boolean_5 INTEGER,
    filter_enum_1 TEXT,
    filter_enum_2 TEXT,
    filter_enum_3 TEXT,
    filter_enum_4 TEXT,
    filter_enum_5 TEXT,
    filter_location_1 TEXT,
    filter_location_2 TEXT,
    filter_location_3 TEXT,
    filter_location_4 TEXT,
    filter_location_5 TEXT,
    custom_fields TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (group_template_id) REFERENCES ext_products_group_templates(id)
);

CREATE INDEX IF NOT EXISTS idx_ext_products_groups_user_id ON ext_products_groups(user_id);
CREATE INDEX IF NOT EXISTS idx_ext_products_groups_group_template_id ON ext_products_groups(group_template_id);
CREATE INDEX IF NOT EXISTS idx_ext_products_groups_filter_numeric_1 ON ext_products_groups(filter_numeric_1);
CREATE INDEX IF NOT EXISTS idx_ext_products_groups_filter_numeric_2 ON ext_products_groups(filter_numeric_2);
CREATE INDEX IF NOT EXISTS idx_ext_products_groups_filter_text_1 ON ext_products_groups(filter_text_1);
CREATE INDEX IF NOT EXISTS idx_ext_products_groups_filter_text_2 ON ext_products_groups(filter_text_2);
CREATE INDEX IF NOT EXISTS idx_ext_products_groups_filter_boolean_1 ON ext_products_groups(filter_boolean_1);
CREATE INDEX IF NOT EXISTS idx_ext_products_groups_filter_enum_1 ON ext_products_groups(filter_enum_1);

CREATE TABLE IF NOT EXISTS ext_products_product_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT,
    description TEXT,
    category TEXT,
    icon TEXT,
    filter_fields_schema TEXT,
    custom_fields_schema TEXT,
    pricing_templates TEXT,
    billing_mode TEXT DEFAULT 'instant' NOT NULL,
    billing_type TEXT DEFAULT 'one-time' NOT NULL,
    billing_recurring_interval TEXT,
    billing_recurring_interval_count INTEGER DEFAULT 1,
    status TEXT DEFAULT 'active',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ext_products_products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER NOT NULL,
    product_template_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    base_price REAL,
    base_price_cents INTEGER,
    currency TEXT DEFAULT 'USD',
    -- Filter columns
    filter_numeric_1 REAL,
    filter_numeric_2 REAL,
    filter_numeric_3 REAL,
    filter_numeric_4 REAL,
    filter_numeric_5 REAL,
    filter_text_1 TEXT,
    filter_text_2 TEXT,
    filter_text_3 TEXT,
    filter_text_4 TEXT,
    filter_text_5 TEXT,
    filter_boolean_1 INTEGER,
    filter_boolean_2 INTEGER,
    filter_boolean_3 INTEGER,
    filter_boolean_4 INTEGER,
    filter_boolean_5 INTEGER,
    filter_enum_1 TEXT,
    filter_enum_2 TEXT,
    filter_enum_3 TEXT,
    filter_enum_4 TEXT,
    filter_enum_5 TEXT,
    filter_location_1 TEXT,
    filter_location_2 TEXT,
    filter_location_3 TEXT,
    filter_location_4 TEXT,
    filter_location_5 TEXT,
    custom_fields TEXT,
    variables TEXT,
    pricing_formula TEXT,
    active INTEGER DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (group_id) REFERENCES ext_products_groups(id),
    FOREIGN KEY (product_template_id) REFERENCES ext_products_product_templates(id)
);

CREATE INDEX IF NOT EXISTS idx_ext_products_products_group_id ON ext_products_products(group_id);
CREATE INDEX IF NOT EXISTS idx_ext_products_products_product_template_id ON ext_products_products(product_template_id);
CREATE INDEX IF NOT EXISTS idx_ext_products_products_filter_numeric_1 ON ext_products_products(filter_numeric_1);
CREATE INDEX IF NOT EXISTS idx_ext_products_products_filter_text_1 ON ext_products_products(filter_text_1);
CREATE INDEX IF NOT EXISTS idx_ext_products_products_filter_boolean_1 ON ext_products_products(filter_boolean_1);
CREATE INDEX IF NOT EXISTS idx_ext_products_products_filter_enum_1 ON ext_products_products(filter_enum_1);
CREATE INDEX IF NOT EXISTS idx_ext_products_products_active ON ext_products_products(active);

CREATE TABLE IF NOT EXISTS ext_products_pricing_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT,
    description TEXT,
    price_formula TEXT NOT NULL,
    condition_formula TEXT,
    variables TEXT,
    category TEXT,
    status TEXT DEFAULT 'active',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ext_products_purchases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    provider TEXT DEFAULT 'stripe',
    provider_session_id TEXT,
    provider_payment_intent_id TEXT,
    provider_subscription_id TEXT,
    line_items TEXT,
    product_metadata TEXT,
    tax_items TEXT,
    amount_cents INTEGER,
    tax_cents INTEGER,
    total_cents INTEGER,
    currency TEXT DEFAULT 'USD',
    status TEXT DEFAULT 'pending',
    requires_approval INTEGER DEFAULT 0,
    approved_at DATETIME,
    approved_by TEXT,
    refunded_at DATETIME,
    refund_reason TEXT,
    refund_amount INTEGER,
    cancelled_at DATETIME,
    cancel_reason TEXT,
    success_url TEXT,
    cancel_url TEXT,
    customer_email TEXT,
    customer_name TEXT,
    billing_address TEXT,
    shipping_address TEXT,
    payment_method_types TEXT,
    expires_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ext_products_purchases_user_id ON ext_products_purchases(user_id);
CREATE INDEX IF NOT EXISTS idx_ext_products_purchases_provider_session_id ON ext_products_purchases(provider_session_id);
CREATE INDEX IF NOT EXISTS idx_ext_products_purchases_provider_payment_intent_id ON ext_products_purchases(provider_payment_intent_id);
CREATE INDEX IF NOT EXISTS idx_ext_products_purchases_provider_subscription_id ON ext_products_purchases(provider_subscription_id);
CREATE INDEX IF NOT EXISTS idx_ext_products_purchases_status ON ext_products_purchases(status);
