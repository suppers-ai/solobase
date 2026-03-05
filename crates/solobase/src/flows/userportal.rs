pub const JSON: &str = r#"{
    "id": "userportal",
    "summary": "User portal config",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/ext/userportal/config", "methods": ["GET"] }
        ]
    },
    "root": {
        "flow": "@wafer/infra",
        "next": [
            { "block": "@solobase/userportal" }
        ]
    }
}"#;
