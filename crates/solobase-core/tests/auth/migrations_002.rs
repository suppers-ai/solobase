//! Apply migrations through 002; verify the four reserved orgs are seeded and
//! that re-applying doesn't error (idempotency).

use crate::common::MigrationTestCtx;
use solobase_core::blocks::auth::migrations;
use wafer_core::clients::database as db;

const EXPECTED_RESERVED: &[&str] = &["solobase", "suppers-ai", "wafer", "wafer-run"];

#[tokio::test]
async fn migration_002_seeds_four_reserved_orgs_idempotently() {
    let ctx = MigrationTestCtx::new();
    migrations::apply(&ctx).await.expect("first apply");
    migrations::apply(&ctx)
        .await
        .expect("second apply must succeed (idempotent)");

    let rows = db::query_raw(
        &ctx,
        "SELECT name FROM suppers_ai__auth__orgs WHERE is_reserved = 1 ORDER BY name",
        &[],
    )
    .await
    .expect("query reserved orgs");

    let names: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            r.data
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();

    assert_eq!(
        names,
        EXPECTED_RESERVED
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        "expected exactly the four reserved orgs, got {names:?}"
    );
}
