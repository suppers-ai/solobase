/// The top-level flow that wafer-run/http-listener dispatches all requests to.
///
/// API requests are routed to suppers-ai/router, which delegates to the
/// shared solobase-core pipeline (JWT validation, feature gates, admin checks,
/// block dispatch). Non-API requests fall through to wafer-run/web for SPA serving.
///
/// Infrastructure blocks (security headers, CORS, rate limiting, monitoring)
/// are applied inline before routing.
pub const JSON: &str = r#"{
    "id": "site-main",
    "summary": "Top-level HTTP dispatch — API router + frontend SPA",
    "config": { "on_error": "stop" },
    "root": {
        "block": "wafer-run/security-headers",
        "next": [{
            "block": "wafer-run/cors",
            "next": [{
                "block": "wafer-run/readonly-guard",
                "next": [{
                    "block": "wafer-run/ip-rate-limit",
                    "next": [{
                        "block": "wafer-run/monitoring",
                        "next": [
                            { "match": "*:/auth/**",       "block": "suppers-ai/router" },
                            { "match": "*:/internal/**",   "block": "suppers-ai/router" },
                            { "match": "*:/admin/**",      "block": "suppers-ai/router" },
                            { "match": "*:/settings/**",   "block": "suppers-ai/router" },
                            { "match": "*:/storage/**",    "block": "suppers-ai/router" },
                            { "match": "*:/b/**",          "block": "suppers-ai/router" },
                            { "match": "*:/profile/**",    "block": "suppers-ai/router" },
                            { "match": "*:/health",        "block": "suppers-ai/router" },
                            { "match": "*:/nav",           "block": "suppers-ai/router" },
                            { "match": "*:/debug/**",      "block": "suppers-ai/router" },
                            { "match": "*:/**",            "block": "wafer-run/web", "config": { "web_root": "./frontend/build", "web_spa": "true", "web_index": "index.html" } }
                        ]
                    }]
                }]
            }]
        }]
    }
}"#;
