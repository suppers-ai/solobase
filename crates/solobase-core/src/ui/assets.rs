//! Embedded static assets — CSS and JS.
//!
//! Asset URLs include a content hash for cache busting:
//! `/static/app-{hash}.css` and `/static/htmx-{hash}.min.js`

use std::sync::OnceLock;

/// The main CSS stylesheet (all design system styles combined).
pub fn css() -> &'static str {
    include_str!("assets/app.css")
}

/// htmx 2.x minified JS.
pub fn htmx_js() -> &'static str {
    include_str!("assets/htmx.min.js")
}

/// Short content hash (first 8 chars of hex SHA-256) for cache busting.
fn short_hash(content: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(content);
    hash.iter().take(4).map(|b| format!("{b:02x}")).collect()
}

/// CSS URL with content hash, e.g. `/static/app-a1b2c3d4.css`
pub fn css_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| format!("/b/static/app-{}.css", short_hash(css().as_bytes())))
}

/// htmx JS URL with content hash, e.g. `/static/htmx-a1b2c3d4.min.js`
pub fn htmx_js_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| format!("/b/static/htmx-{}.min.js", short_hash(htmx_js().as_bytes())))
}

/// Small inline JS for toast notifications (triggered by htmx HX-Trigger).
pub fn toast_js() -> &'static str {
    r#"
document.body.addEventListener("showToast", function(e) {
    var d = e.detail || {};
    var c = document.getElementById("toast-container");
    if (!c) return;
    var t = document.createElement("div");
    t.className = "toast toast-" + (d.type || "info");
    t.innerHTML = '<span>' + (d.message || '') + '</span><button class="toast-dismiss" onclick="this.parentElement.remove()">&times;</button>';
    c.appendChild(t);
    setTimeout(function() { t.remove(); }, 4000);
});
"#
}

/// Small inline JS for sidebar toggle (mobile menu + collapse).
pub fn sidebar_js() -> &'static str {
    r#"
function toggleMobileMenu() {
    document.querySelector('.sidebar-container').classList.toggle('active');
}
function toggleSidebar() {
    document.querySelector('.sidebar').classList.toggle('collapsed');
    localStorage.setItem('sidebar-collapsed', document.querySelector('.sidebar').classList.contains('collapsed'));
}
(function() {
    if (localStorage.getItem('sidebar-collapsed') === 'true') {
        var s = document.querySelector('.sidebar');
        if (s) s.classList.add('collapsed');
    }
})();
"#
}

/// Small inline JS for modal close (Escape key + overlay click).
pub fn modal_js() -> &'static str {
    r#"
document.addEventListener("keydown", function(e) {
    if (e.key === "Escape") {
        var m = document.querySelector('.modal-overlay:not([hidden])');
        if (m) m.setAttribute("hidden", "");
    }
});
function openModal(id) {
    var m = document.getElementById(id);
    if (m) m.removeAttribute("hidden");
}
function closeModal(id) {
    var m = document.getElementById(id);
    if (m) m.setAttribute("hidden", "");
}
document.body.addEventListener("closeModal", function(e) {
    var d = e.detail || {};
    if (d.id) closeModal(d.id);
});
"#
}
