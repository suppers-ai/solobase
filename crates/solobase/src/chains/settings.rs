pub const JSON: &str = r#"{
    "id": "settings",
    "summary": "Settings routes",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/settings",       "path_prefix": true },
            { "path": "/admin/settings", "path_prefix": true }
        ]
    },
    "root": {
        "chain": "http-infra",
        "next": [
            {
                "match": "*:/admin/settings/**",
                "chain": "admin-pipe",
                "next": [
                    { "block": "admin-feature" }
                ]
            },
            {
                "match": "*:/settings/**",
                "chain": "auth-pipe",
                "next": [
                    { "block": "admin-feature" }
                ]
            }
        ]
    }
}"#;
