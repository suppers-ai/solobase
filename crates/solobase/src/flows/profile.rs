pub const JSON: &str = r#"{
    "id": "profile",
    "summary": "Profile sections",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/profile/sections", "methods": ["GET"] }
        ]
    },
    "root": {
        "flow": "@wafer/infra",
        "next": [
            {
                "flow": "@wafer/auth-pipe",
                "next": [
                    { "block": "@solobase/profile" }
                ]
            }
        ]
    }
}"#;
