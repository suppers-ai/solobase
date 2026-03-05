/// The top-level flow that @wafer/http-listener dispatches all requests to.
/// Routes to the appropriate feature flow based on path matching.
/// The @wafer/web block serves the frontend SPA as a final fallback.
pub const JSON: &str = r#"{
    "id": "site-main",
    "summary": "Top-level HTTP dispatch — routes to feature flows and frontend",
    "config": { "on_error": "stop" },
    "root": {
        "flow": "@wafer/infra",
        "next": [
            { "match": "*:/auth/**",                 "flow": "auth" },
            { "match": "*:/internal/oauth/**",       "flow": "auth" },
            { "match": "*:/admin/users/**",          "flow": "admin" },
            { "match": "*:/admin/database/**",       "flow": "admin" },
            { "match": "*:/admin/logs/**",           "flow": "admin" },
            { "match": "*:/admin/iam/**",            "flow": "admin" },
            { "match": "*:/admin/wafer/**",          "flow": "admin" },
            { "match": "*:/admin/custom-tables/**",  "flow": "admin" },
            { "match": "*:/admin/monitoring/**",     "flow": "monitoring" },
            { "match": "*:/admin/settings/**",       "flow": "settings" },
            { "match": "*:/settings/**",             "flow": "settings" },
            { "match": "*:/admin/storage/**",        "flow": "files" },
            { "match": "*:/admin/ext/cloudstorage/**", "flow": "files" },
            { "match": "*:/storage/**",              "flow": "files" },
            { "match": "*:/ext/cloudstorage/**",     "flow": "files" },
            { "match": "*:/admin/legalpages/**",     "flow": "legalpages" },
            { "match": "*:/ext/legalpages/**",       "flow": "legalpages" },
            { "match": "*:/admin/ext/products/**",   "flow": "products" },
            { "match": "*:/ext/products/**",         "flow": "products" },
            { "match": "*:/ext/userportal/**",       "flow": "userportal" },
            { "match": "*:/profile/**",              "flow": "profile" },
            { "match": "*:/health",                  "flow": "system" },
            { "match": "*:/debug/**",                "flow": "system" },
            { "match": "*:/nav",                     "flow": "system" },
            { "match": "*:/**",                      "block": "@wafer/web", "config": { "web_root": "./frontend/build", "web_spa": "true", "web_index": "index.html" } }
        ]
    }
}"#;
