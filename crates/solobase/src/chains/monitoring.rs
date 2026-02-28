pub const JSON: &str = r#"{
    "id": "monitoring",
    "summary": "Monitoring dashboard",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/admin/monitoring", "path_prefix": true }
        ]
    },
    "root": {
        "chain": "http-infra",
        "next": [
            {
                "chain": "admin-pipe",
                "next": [
                    { "block": "monitoring-feature" }
                ]
            }
        ]
    }
}"#;
