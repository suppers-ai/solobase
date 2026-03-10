pub const JSON: &str = r#"{
    "id": "legalpages",
    "summary": "Legal pages routes",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/b/legalpages/terms",   "methods": ["GET"] },
            { "path": "/b/legalpages/privacy",  "methods": ["GET"] },
            { "path": "/admin/legalpages",        "path_prefix": true },
            { "path": "/b/legalpages",          "path_prefix": true }
        ]
    },
    "root": {
        "flow": "@wafer/infra",
        "next": [
            { "match": "GET:/b/legalpages/terms",   "block": "@solobase/legalpages" },
            { "match": "GET:/b/legalpages/privacy",  "block": "@solobase/legalpages" },
            {
                "match": "*:/admin/legalpages/**",
                "flow": "@wafer/admin-pipe",
                "next": [
                    { "block": "@solobase/legalpages" }
                ]
            },
            {
                "match": "*:/b/legalpages/**",
                "flow": "@wafer/admin-pipe",
                "next": [
                    { "block": "@solobase/legalpages" }
                ]
            }
        ]
    }
}"#;
