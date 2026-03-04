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
        "flow": "http-infra",
        "next": [
            {
                "match": "*:/admin/settings/**",
                "flow": "admin-pipe",
                "next": [
                    { "block": "admin-feature" }
                ]
            },
            {
                "match": "*:/settings/**",
                "flow": "auth-pipe",
                "next": [
                    { "block": "admin-feature" }
                ]
            }
        ]
    }
}"#;
