pub const JSON: &str = r#"{
    "id": "products",
    "summary": "Products extension routes",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/b/products/webhooks", "methods": ["POST"] },
            { "path": "/b/products",          "path_prefix": true },
            { "path": "/admin/b/products",    "path_prefix": true }
        ]
    },
    "root": {
        "flow": "@wafer/infra",
        "next": [
            { "match": "POST:/b/products/webhooks", "block": "@solobase/products" },
            {
                "match": "*:/admin/b/products/**",
                "flow": "@wafer/admin-pipe",
                "next": [
                    { "block": "@solobase/products" }
                ]
            },
            {
                "match": "*:/b/products/**",
                "flow": "@wafer/auth-pipe",
                "next": [
                    { "block": "@solobase/products" }
                ]
            }
        ]
    }
}"#;
