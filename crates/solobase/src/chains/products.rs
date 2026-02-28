pub const JSON: &str = r#"{
    "id": "products",
    "summary": "Products extension routes",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/ext/products/webhooks", "methods": ["POST"] },
            { "path": "/ext/products",          "path_prefix": true },
            { "path": "/admin/ext/products",    "path_prefix": true }
        ]
    },
    "root": {
        "chain": "http-infra",
        "next": [
            { "match": "POST:/ext/products/webhooks", "block": "products-feature" },
            {
                "match": "*:/admin/ext/products/**",
                "chain": "admin-pipe",
                "next": [
                    { "block": "products-feature" }
                ]
            },
            {
                "match": "*:/ext/products/**",
                "chain": "auth-pipe",
                "next": [
                    { "block": "products-feature" }
                ]
            }
        ]
    }
}"#;
