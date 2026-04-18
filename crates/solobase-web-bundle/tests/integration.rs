use solobase_web_bundle::run;
use std::fs;
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pkg-in")
}

#[test]
fn end_to_end_renames_rewrites_and_templates() {
    let tmp = tempfile::tempdir().unwrap();
    copy_dir(&fixture_path(), tmp.path());

    run(tmp.path(), tmp.path(), /* dev */ false).expect("bundler ok");

    let manifest_body = fs::read_to_string(tmp.path().join("asset-manifest.json")).unwrap();
    assert!(manifest_body.contains("\"buildId\""));
    assert!(manifest_body.contains("\"solobase_web.js\""));
    assert!(manifest_body.contains("\"solobase_web_bg.wasm\""));

    let entries: Vec<String> = fs::read_dir(tmp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert!(
        entries
            .iter()
            .any(|n| n.starts_with("solobase_web-") && n.ends_with(".js")),
        "missing hashed JS in {:?}",
        entries
    );
    assert!(entries
        .iter()
        .any(|n| n.starts_with("solobase_web_bg-") && n.ends_with(".wasm")));
    assert!(!entries.iter().any(|n| n == "solobase_web.js"));
    assert!(!entries.iter().any(|n| n == "solobase_web_bg.wasm"));

    let sw = fs::read_to_string(tmp.path().join("sw.js")).unwrap();
    assert!(sw.contains("from '/solobase_web-"), "sw.js = {sw}");
    assert!(!sw.contains("__WASM_JS__"));
    assert!(!sw.contains("__BUILD_ID__"));

    let glue_name = entries
        .iter()
        .find(|n| n.starts_with("solobase_web-") && n.ends_with(".js"))
        .unwrap();
    let glue = fs::read_to_string(tmp.path().join(glue_name)).unwrap();
    assert!(glue.contains("solobase_web_bg-"), "glue = {glue}");
    assert!(!glue.contains("'solobase_web_bg.wasm'"));
}

#[test]
fn deterministic_across_runs() {
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    copy_dir(&fixture_path(), tmp1.path());
    copy_dir(&fixture_path(), tmp2.path());
    solobase_web_bundle::run(tmp1.path(), tmp1.path(), false).unwrap();
    solobase_web_bundle::run(tmp2.path(), tmp2.path(), false).unwrap();

    let m1 = fs::read_to_string(tmp1.path().join("asset-manifest.json")).unwrap();
    let m2 = fs::read_to_string(tmp2.path().join("asset-manifest.json")).unwrap();
    let v1: serde_json::Value = serde_json::from_str(&m1).unwrap();
    let v2: serde_json::Value = serde_json::from_str(&m2).unwrap();
    assert_eq!(v1.get("assets"), v2.get("assets"));
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) {
    for entry in fs::read_dir(src).unwrap() {
        let e = entry.unwrap();
        let to = dst.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            fs::create_dir_all(&to).unwrap();
            copy_dir(&e.path(), &to);
        } else {
            fs::copy(e.path(), to).unwrap();
        }
    }
}
