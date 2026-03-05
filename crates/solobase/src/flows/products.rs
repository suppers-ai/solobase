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
        "flow": "@wafer/infra",
        "next": [
            { "match": "POST:/ext/products/webhooks", "block": "@solobase/products" },
            {
                "match": "*:/admin/ext/products/**",
                "flow": "@wafer/admin-pipe",
                "next": [
                    { "block": "@solobase/products" }
                ]
            },
            {
                "match": "*:/ext/products/**",
                "flow": "@wafer/auth-pipe",
                "next": [
                    { "block": "@solobase/products" }
                ]
            }
        ]
    }
}"#;
