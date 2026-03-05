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
        "flow": "@wafer/infra",
        "next": [
            { "match": "GET:/storage/direct/{token}", "block": "@solobase/files" },
            {
                "match": "*:/admin/storage/**",
                "flow": "@wafer/admin-pipe",
                "next": [
                    { "block": "@solobase/files" }
                ]
            },
            {
                "match": "*:/admin/ext/cloudstorage/**",
                "flow": "@wafer/admin-pipe",
                "next": [
                    { "block": "@solobase/files" }
                ]
            },
            {
                "match": "*:/storage/**",
                "flow": "@wafer/auth-pipe",
                "next": [
                    { "block": "@solobase/files" }
                ]
            },
            {
                "match": "*:/ext/cloudstorage/**",
                "flow": "@wafer/auth-pipe",
                "next": [
                    { "block": "@solobase/files" }
                ]
            }
        ]
    }
}"#;
