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
        "flow": "@wafer/infra",
        "next": [
            {
                "match": "*:/admin/settings/**",
                "flow": "@wafer/admin-pipe",
                "next": [
                    { "block": "@solobase/admin" }
                ]
            },
            {
                "match": "*:/settings/**",
                "flow": "@wafer/auth-pipe",
                "next": [
                    { "block": "@solobase/admin" }
                ]
            }
        ]
    }
}"#;
