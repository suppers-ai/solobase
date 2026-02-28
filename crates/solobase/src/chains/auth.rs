pub const JSON: &str = r#"{
    "id": "auth",
    "summary": "Authentication routes",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/auth/login",           "methods": ["POST"] },
            { "path": "/auth/signup",          "methods": ["POST"] },
            { "path": "/auth/refresh",         "methods": ["POST"] },
            { "path": "/auth/me",              "methods": ["GET", "PATCH"] },
            { "path": "/auth/logout",          "methods": ["POST"] },
            { "path": "/auth/change-password", "methods": ["POST"] },
            { "path": "/auth/api-keys",        "path_prefix": true },
            { "path": "/auth/oauth",           "path_prefix": true },
            { "path": "/internal/oauth/sync-user", "methods": ["POST"] }
        ]
    },
    "root": {
        "chain": "http-infra",
        "next": [
            { "match": "POST:/auth/login",                "block": "auth-feature" },
            { "match": "POST:/auth/signup",               "block": "auth-feature" },
            { "match": "POST:/auth/refresh",              "block": "auth-feature" },
            { "match": "*:/auth/oauth/**",                "block": "auth-feature" },
            { "match": "POST:/internal/oauth/sync-user",  "block": "auth-feature" },
            {
                "match": "*:/auth/**",
                "chain": "auth-pipe",
                "next": [
                    { "block": "auth-feature" }
                ]
            }
        ]
    }
}"#;
