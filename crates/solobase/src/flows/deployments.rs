pub const JSON: &str = r#"{
    "id": "deployments",
    "summary": "Deployments extension routes",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/ext/deployments",          "path_prefix": true },
            { "path": "/admin/ext/deployments",    "path_prefix": true }
        ]
    },
    "root": {
        "flow": "@wafer/infra",
        "next": [
            {
                "match": "*:/admin/ext/deployments/**",
                "flow": "@wafer/admin-pipe",
                "next": [
                    { "block": "@solobase/deployments" }
                ]
            },
            {
                "match": "*:/ext/deployments/**",
                "flow": "@wafer/auth-pipe",
                "next": [
                    { "block": "@solobase/deployments" }
                ]
            }
        ]
    }
}"#;
