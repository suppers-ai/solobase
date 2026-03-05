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
        "flow": "@wafer/infra",
        "next": [
            { "match": "POST:/auth/login",                "block": "@solobase/auth" },
            { "match": "POST:/auth/signup",               "block": "@solobase/auth" },
            { "match": "POST:/auth/refresh",              "block": "@solobase/auth" },
            { "match": "*:/auth/oauth/**",                "block": "@solobase/auth" },
            { "match": "POST:/internal/oauth/sync-user",  "block": "@solobase/auth" },
            {
                "match": "*:/auth/**",
                "flow": "@wafer/auth-pipe",
                "next": [
                    { "block": "@solobase/auth" }
                ]
            }
        ]
    }
}"#;
