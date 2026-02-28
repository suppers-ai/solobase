pub const JSON: &str = r#"{
    "id": "legalpages",
    "summary": "Legal pages routes",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/ext/legalpages/terms",   "methods": ["GET"] },
            { "path": "/ext/legalpages/privacy",  "methods": ["GET"] },
            { "path": "/admin/legalpages",        "path_prefix": true },
            { "path": "/ext/legalpages",          "path_prefix": true }
        ]
    },
    "root": {
        "chain": "http-infra",
        "next": [
            { "match": "GET:/ext/legalpages/terms",   "block": "legalpages-feature" },
            { "match": "GET:/ext/legalpages/privacy",  "block": "legalpages-feature" },
            {
                "match": "*:/admin/legalpages/**",
                "chain": "admin-pipe",
                "next": [
                    { "block": "legalpages-feature" }
                ]
            },
            {
                "match": "*:/ext/legalpages/**",
                "chain": "admin-pipe",
                "next": [
                    { "block": "legalpages-feature" }
                ]
            }
        ]
    }
}"#;
