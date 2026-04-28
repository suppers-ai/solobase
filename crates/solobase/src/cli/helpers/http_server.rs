//! Minimal static-file HTTP server shared by sealed × web and embed × web
//! `serve` flows. Reads request line, serves matching file from `dir`.
//! Development convenience — not a production server.

use std::path::Path;

use anyhow::Result;

pub async fn serve_static(dir: &Path, port: u16) -> Result<()> {
    use tokio::net::TcpListener;
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let dir = dir.to_path_buf();
    loop {
        let (mut socket, _) = listener.accept().await?;
        let dir = dir.clone();
        tokio::spawn(async move {
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
