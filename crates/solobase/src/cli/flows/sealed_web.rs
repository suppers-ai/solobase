//! Sealed × web: assemble a static dist/ from the bundled solobase-web
//! wasm + the user's frontend overlays + any blocks/.

use std::path::Path;

use anyhow::{anyhow, Result};

use crate::cli::helpers::{blocks, overlays, wasm};
use crate::cli::legacy_config;

pub async fn build(repo_root: &Path, release: bool) -> Result<()> {
    // 1. Discover and build user blocks (if any).
    blocks::build_all(repo_root)?;

    // 2. Prepare dist directory.
    let dist = repo_root.join("dist");
    if dist.exists() {
        std::fs::remove_dir_all(&dist).map_err(|e| anyhow!("clean dist/: {e}"))?;
    }
    std::fs::create_dir_all(&dist).map_err(|e| anyhow!("create dist/: {e}"))?;

    // 3. Resolve and write the solobase-web wasm.
    let wasm_bytes = wasm::resolve_solobase_web_wasm()?;
    let wasm_path = dist.join("solobase-web.wasm");
    std::fs::write(&wasm_path, &*wasm_bytes)
        .map_err(|e| anyhow!("write {wasm_path:?}: {e}"))?;

    // 4. Run the bundler — content-hash assets + render templates.
    //    This calls solobase_browser::tools::bundle::run, which writes the
    //    static shell (index.html, sw.js, loader.js) into dist/.
    let cfg = legacy_config::find_and_load(repo_root).ok();
    let app = match cfg.as_ref() {
        Some((c, _)) => solobase_browser::tools::bundle::AppConfig {
            app_name: Some(c.app.name.clone()),
            app_title: Some(c.app.title.clone()),
            boot_redirect: Some(c.app.boot_redirect.clone()),
            extra_bypass_prefix: c.assets.extra_bypass_prefix.clone(),
        },
        None => solobase_browser::tools::bundle::AppConfig {
            app_name: None,
            app_title: None,
            boot_redirect: None,
            extra_bypass_prefix: vec![],
        },
    };

    solobase_browser::assets::write_to(&dist)?;
    solobase_browser::tools::bundle::run(&dist, repo_root, !release, app)?;

    // 5. Apply overlays from solobase.toml if present.
    if let Some((cfg, root)) = cfg {
        overlays::apply_overlays(&cfg, &root, &dist)?;
    }

    let profile = if release { "release" } else { "dev" };
    println!("built sealed × web ({profile}) → dist/");
    Ok(())
}

pub async fn serve(repo_root: &Path, release: bool, port: Option<u16>) -> Result<()> {
    build(repo_root, release).await?;
    let port = port.unwrap_or(8080);
    let dist = repo_root.join("dist");
    eprintln!("serving {} on http://127.0.0.1:{port}", dist.display());
    serve_static(&dist, port).await
}

async fn serve_static(dir: &Path, port: u16) -> Result<()> {
    use tokio::net::TcpListener;
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let dir = dir.to_path_buf();
    loop {
        let (mut socket, _) = listener.accept().await?;
        let dir = dir.clone();
        tokio::spawn(async move {
            // Minimal static-file server. Reads request line, serves the
            // matching file from `dir`. 404 otherwise. No HTTP/1.1 parser
            // niceties; this is a development convenience.
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 1024];
            let n = match socket.read(&mut buf).await {
                Ok(n) => n,
                Err(_) => return,
            };
            let req = String::from_utf8_lossy(&buf[..n]);
            let path = req
                .lines()
                .next()
                .and_then(|l| l.split_whitespace().nth(1))
                .unwrap_or("/");
            let file_path = if path == "/" {
                dir.join("index.html")
            } else {
                dir.join(path.trim_start_matches('/'))
            };
            let body = tokio::fs::read(&file_path).await;
            let resp = match body {
                Ok(b) => {
                    let mime = mime_guess(&file_path);
                    let mut out = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\n\r\n",
                        b.len()
                    )
                    .into_bytes();
                    out.extend(b);
                    out
                }
                Err(_) => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_vec(),
            };
            let _ = socket.write_all(&resp).await;
        });
    }
}

fn mime_guess(p: &Path) -> &'static str {
    match p.extension().and_then(|s| s.to_str()) {
        Some("html") => "text/html",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("wasm") => "application/wasm",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}
