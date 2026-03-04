pub const JSON: &str = r#"{
    "id": "system",
    "summary": "System infrastructure routes",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/health",      "methods": ["GET"] },
            { "path": "/debug/time",  "methods": ["GET"] },
            { "path": "/nav",         "methods": ["GET"] }
        ]
    },
    "root": {
        "flow": "http-infra",
        "next": [
            { "match": "GET:/health",     "block": "system-feature" },
            { "match": "GET:/debug/time", "block": "system-feature" },
            {
                "match": "GET:/nav",
                "flow": "auth-pipe",
                "next": [
                    { "block": "system-feature" }
                ]
            }
        ]
    }
}"#;
