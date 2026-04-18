pub mod build_id;
pub mod hash;
pub mod manifest;
pub mod rename;
pub mod template;

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

/// Input files we content-hash. Keys are logical names used by templates.
const HASHED_ASSETS: &[(&str, &str)] = &[
    ("solobase_web.js", "solobase_web.js"),
    ("solobase_web_bg.wasm", "solobase_web_bg.wasm"),
    ("sql-wasm-esm.js", "sql-wasm-esm.js"),
    ("sql-wasm.wasm", "sql-wasm.wasm"),
];

/// Cross-references inside hashed files that we rewrite after renaming.
/// (source-logical, quote-char, target-logical, replace-all)
///
/// `replace_all = true` — the UMD bundle embeds the path multiple times.
/// `replace_all = false` — wasm-bindgen glue has exactly one reference.
const REWRITES: &[(&str, char, &str, bool)] = &[
    ("solobase_web.js", '\'', "solobase_web_bg.wasm", false),
    // sql.js is a minified UMD bundle that contains several copies of the
    // WASM filename (locateFile fallback + inline checks). Replace them all.
    ("sql-wasm-esm.js", '"', "sql-wasm.wasm", true),
];

pub fn run(pkg_dir: &Path, repo_dir: &Path, dev: bool) -> Result<()> {
    if dev {
        return run_dev(pkg_dir);
    }
    let mut hashes: BTreeMap<String, String> = BTreeMap::new();
    let mut renamed: BTreeMap<String, std::path::PathBuf> = BTreeMap::new();

    // 1. Compute hashes and rename each asset.
    for (logical, filename) in HASHED_ASSETS {
        let src = pkg_dir.join(filename);
        let bytes = std::fs::read(&src).with_context(|| format!("reading {}", src.display()))?;
        let hash = hash::short_hash(&bytes);
        let new_path = rename::rename_with_hash(&src, &hash)?;
        hashes.insert((*logical).to_string(), hash);
        renamed.insert((*logical).to_string(), new_path);
    }

    // 2. Rewrite cross-references.
    for (source_logical, quote, target_logical, replace_all) in REWRITES {
        let source_path = renamed
            .get(*source_logical)
            .ok_or_else(|| anyhow::anyhow!("missing renamed source: {source_logical}"))?;
        let old_name = HASHED_ASSETS
            .iter()
            .find(|(l, _)| *l == *target_logical)
            .unwrap()
            .1;
        let new_name = renamed
            .get(*target_logical)
            .ok_or_else(|| anyhow::anyhow!("missing renamed target: {target_logical}"))?
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let old_literal = format!("{quote}{old_name}{quote}");
        let new_literal = format!("{quote}{new_name}{quote}");
        if *replace_all {
            rename::rewrite_all(source_path, &old_literal, &new_literal)?;
        } else {
            rename::rewrite_literal(source_path, &old_literal, &new_literal)?;
        }
    }

    // 3. buildId.
    let asset_hashes_ordered: Vec<&str> = HASHED_ASSETS
        .iter()
        .map(|(l, _)| hashes.get(*l).unwrap().as_str())
        .collect();
    let build_id = build_id::build_id(repo_dir, &asset_hashes_ordered);

    // 4. Manifest.
    let mut manifest_assets = BTreeMap::new();
    for (logical, _) in HASHED_ASSETS {
        let new_path = renamed.get(*logical).unwrap();
        let url = format!("/{}", new_path.file_name().unwrap().to_string_lossy());
        manifest_assets.insert((*logical).to_string(), url);
    }
    let manifest = manifest::AssetManifest {
        build_id: build_id.clone(),
        assets: manifest_assets.clone(),
    };
    manifest.write(&pkg_dir.join("asset-manifest.json"))?;

    // 5. Render templates.
    let mut vars: BTreeMap<String, String> = BTreeMap::new();
    vars.insert("BUILD_ID".to_string(), build_id);
    for (logical, url) in &manifest_assets {
        vars.insert(template_key(logical), url.clone());
    }
    render_if_exists(pkg_dir, "sw.js.tmpl", "sw.js", &vars)?;
    render_if_exists(pkg_dir, "index.html.tmpl", "index.html", &vars)?;

    Ok(())
}

fn run_dev(pkg_dir: &Path) -> Result<()> {
    let mut vars: BTreeMap<String, String> = BTreeMap::new();
    vars.insert("BUILD_ID".to_string(), "dev".to_string());
    for (logical, filename) in HASHED_ASSETS {
        vars.insert(template_key(logical), format!("/{filename}"));
    }
    render_if_exists(pkg_dir, "sw.js.tmpl", "sw.js", &vars)?;
    render_if_exists(pkg_dir, "index.html.tmpl", "index.html", &vars)?;
    Ok(())
}

fn render_if_exists(
    pkg_dir: &Path,
    src_name: &str,
    out_name: &str,
    vars: &BTreeMap<String, String>,
) -> Result<()> {
    let src = pkg_dir.join(src_name);
    if !src.exists() {
        return Ok(());
    }
    template::render_to_file(&src, &pkg_dir.join(out_name), vars)?;
    std::fs::remove_file(&src).ok();
    Ok(())
}

fn template_key(logical: &str) -> String {
    match logical {
        "solobase_web.js" => "WASM_JS".into(),
        "solobase_web_bg.wasm" => "WASM_BIN".into(),
        "sql-wasm-esm.js" => "SQL_JS".into(),
        "sql-wasm.wasm" => "SQL_WASM".into(),
        other => panic!("unknown logical asset: {other}"),
    }
}
