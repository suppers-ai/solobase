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
        "chain": "http-infra",
        "next": [
            {
                "chain": "auth-pipe",
                "next": [
                    { "block": "profile-feature" }
                ]
            }
        ]
    }
}"#;
