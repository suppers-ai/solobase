pub const JSON: &str = r#"{
    "id": "protected-pipe",
    "summary": "Protected route authentication pipeline (alias for auth-pipe)",
    "config": { "on_error": "stop" },
    "root": {
        "block": "@wafer/auth"
    }
}"#;
