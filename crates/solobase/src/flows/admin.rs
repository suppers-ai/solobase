pub const JSON: &str = r#"{
    "id": "admin",
    "summary": "Admin management",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/admin/users",         "path_prefix": true },
            { "path": "/admin/database",      "path_prefix": true },
            { "path": "/admin/logs",          "path_prefix": true },
            { "path": "/admin/iam",           "path_prefix": true },
            { "path": "/admin/wafer",        "path_prefix": true },
            { "path": "/admin/waffle",       "path_prefix": true },
            { "path": "/admin/custom-tables", "path_prefix": true }
        ]
    },
    "root": {
        "flow": "http-infra",
        "next": [
            {
                "flow": "admin-pipe",
                "next": [
                    { "block": "admin-feature" }
                ]
            }
        ]
    }
}"#;
