pub const JSON: &str = r#"{
    "id": "web-site",
    "summary": "Static website serving",
    "config": { "on_error": "stop" },
    "root": {
        "flow": "http-infra",
        "next": [
            { "block": "web-feature" }
        ]
    }
}"#;
