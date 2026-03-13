/// The top-level flow that @wafer/http-listener dispatches all requests to.
///
/// API requests are routed to @solobase/router, which delegates to the
/// shared solobase-core pipeline (JWT validation, feature gates, admin checks,
/// block dispatch). Non-API requests fall through to @wafer/web for SPA serving.
///
/// Infrastructure blocks (security headers, CORS, rate limiting, monitoring)
/// are applied inline before routing.
pub const JSON: &str = r#"{
    "id": "site-main",
    "summary": "Top-level HTTP dispatch — API router + frontend SPA",
    "config": { "on_error": "stop" },
    "root": {
        "block": "@wafer/security-headers",
        "next": [{
            "block": "@wafer/cors",
            "next": [{
                "block": "@wafer/readonly-guard",
                "next": [{
                    "block": "@wafer/ip-rate-limit",
                    "next": [{
                        "block": "@wafer/monitoring",
                        "next": [
                            { "match": "*:/auth/**",       "block": "@solobase/router" },
                            { "match": "*:/internal/**",   "block": "@solobase/router" },
                            { "match": "*:/admin/**",      "block": "@solobase/router" },
                            { "match": "*:/settings/**",   "block": "@solobase/router" },
                            { "match": "*:/storage/**",    "block": "@solobase/router" },
                            { "match": "*:/b/**",          "block": "@solobase/router" },
                            { "match": "*:/profile/**",    "block": "@solobase/router" },
                            { "match": "*:/health",        "block": "@solobase/router" },
                            { "match": "*:/nav",           "block": "@solobase/router" },
                            { "match": "*:/debug/**",      "block": "@solobase/router" },
                            { "match": "*:/**",            "block": "@wafer/web", "config": { "web_root": "./frontend/build", "web_spa": "true", "web_index": "index.html" } }
                        ]
                    }]
                }]
            }]
        }]
    }
}"#;
