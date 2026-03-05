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
        "flow": "@wafer/infra",
        "next": [
            { "match": "GET:/health",     "block": "@solobase/system" },
            { "match": "GET:/debug/time", "block": "@solobase/system" },
            {
                "match": "GET:/nav",
                "flow": "@wafer/auth-pipe",
                "next": [
                    { "block": "@solobase/system" }
                ]
            }
        ]
    }
}"#;
