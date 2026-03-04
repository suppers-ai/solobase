pub const JSON: &str = r#"{
    "id": "files",
    "summary": "File storage, sharing, quotas and access logging",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/storage/direct/{token}", "methods": ["GET"] },
            { "path": "/storage",                "path_prefix": true },
            { "path": "/admin/storage",          "path_prefix": true },
            { "path": "/ext/cloudstorage",       "path_prefix": true },
            { "path": "/admin/ext/cloudstorage", "path_prefix": true }
        ]
    },
    "root": {
        "flow": "http-infra",
        "next": [
            { "match": "GET:/storage/direct/{token}", "block": "files-feature" },
            {
                "match": "*:/admin/storage/**",
                "flow": "admin-pipe",
                "next": [
                    { "block": "files-feature" }
                ]
            },
            {
                "match": "*:/admin/ext/cloudstorage/**",
                "flow": "admin-pipe",
                "next": [
                    { "block": "files-feature" }
                ]
            },
            {
                "match": "*:/storage/**",
                "flow": "auth-pipe",
                "next": [
                    { "block": "files-feature" }
                ]
            },
            {
                "match": "*:/ext/cloudstorage/**",
                "flow": "auth-pipe",
                "next": [
                    { "block": "files-feature" }
                ]
            }
        ]
    }
}"#;
