pub const JSON: &str = r#"{
    "id": "deployments",
    "summary": "Deployments extension routes",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/b/deployments",          "path_prefix": true },
            { "path": "/admin/b/deployments",    "path_prefix": true }
        ]
    },
    "root": {
        "flow": "@wafer/infra",
        "next": [
            {
                "match": "*:/admin/b/deployments/**",
                "flow": "@wafer/admin-pipe",
                "next": [
                    { "block": "@solobase/deployments" }
                ]
            },
            {
                "match": "*:/b/deployments/**",
                "flow": "@wafer/auth-pipe",
                "next": [
                    { "block": "@solobase/deployments" }
                ]
            }
        ]
    }
}"#;
