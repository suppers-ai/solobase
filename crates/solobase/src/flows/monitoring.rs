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
        "flow": "@wafer/infra",
        "next": [
            {
                "flow": "@wafer/admin-pipe",
                "next": [
                    { "block": "@solobase/monitoring" }
                ]
            }
        ]
    }
}"#;
