pub const JSON: &str = r#"{
    "id": "legalpages",
    "summary": "Legal pages routes",
    "config": { "on_error": "stop" },
    "http": {
        "routes": [
            { "path": "/ext/legalpages/terms",   "methods": ["GET"] },
            { "path": "/ext/legalpages/privacy",  "methods": ["GET"] },
            { "path": "/admin/legalpages",        "path_prefix": true },
            { "path": "/ext/legalpages",          "path_prefix": true }
        ]
    },
    "root": {
        "flow": "@wafer/infra",
        "next": [
            { "match": "GET:/ext/legalpages/terms",   "block": "@solobase/legalpages" },
            { "match": "GET:/ext/legalpages/privacy",  "block": "@solobase/legalpages" },
            {
                "match": "*:/admin/legalpages/**",
                "flow": "@wafer/admin-pipe",
                "next": [
                    { "block": "@solobase/legalpages" }
                ]
            },
            {
                "match": "*:/ext/legalpages/**",
                "flow": "@wafer/admin-pipe",
                "next": [
                    { "block": "@solobase/legalpages" }
                ]
            }
        ]
    }
}"#;
