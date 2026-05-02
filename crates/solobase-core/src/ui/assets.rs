//! Embedded static assets — CSS and JS.
//!
//! Asset URLs include a content hash for cache busting:
//! `/static/app-{hash}.css` and `/static/htmx-{hash}.min.js`

use std::sync::OnceLock;

const TOKENS_CSS: &str = include_str!("assets/tokens.css");
const BASE_CSS: &str = include_str!("assets/base.css");
const COMPONENTS_CSS: &str = include_str!("assets/components.css");
const LAYOUT_CSS: &str = include_str!("assets/layout.css");

/// Embedded CSS bundle — concatenation of tokens / base / components / layout.
/// Served as one file at the URL returned by `css_url()` so a single
/// `<link rel="stylesheet">` covers everything.
pub fn css_bundle() -> String {
    format!(
        "{}\n{}\n{}\n{}\n",
        TOKENS_CSS, BASE_CSS, COMPONENTS_CSS, LAYOUT_CSS
    )
}

/// The main CSS stylesheet (all design system styles combined).
pub fn css() -> String {
    css_bundle()
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

/// Vanilla JS for the command palette — open/close, fuzzy filter,
/// keyboard navigation. Embedded as a string the same way `toast_js()`
/// and `modal_js()` are.
pub fn palette_js() -> &'static str {
    r#"
(function () {
  if (window.__cmdkInit) return;
  window.__cmdkInit = true;
  const el = document.getElementById('cmdk');
  if (!el) return;
  const input = document.getElementById('cmdk-input');
  const list = document.getElementById('cmdk-list');

  const items = () => Array.from(list.querySelectorAll('.palette__item'));
  let selected = 0;

  function open() {
    el.dataset.open = 'true';
    el.setAttribute('aria-hidden', 'false');
    input.value = '';
    apply('');
    requestAnimationFrame(() => input.focus());
  }
  function close() {
    el.dataset.open = 'false';
    el.setAttribute('aria-hidden', 'true');
  }
  function visibleItems() { return items().filter(i => !i.classList.contains('is-hidden')); }

  function apply(query) {
    const q = query.trim().toLowerCase();
    items().forEach(i => {
      const k = (i.dataset.keywords || '').toLowerCase();
      const match = !q || k.includes(q);
      i.classList.toggle('is-hidden', !match);
      i.setAttribute('aria-selected', 'false');
    });
    const vis = visibleItems();
    selected = 0;
    if (vis[0]) vis[0].setAttribute('aria-selected', 'true');
  }

  function move(delta) {
    const vis = visibleItems();
    if (!vis.length) return;
    vis[selected]?.setAttribute('aria-selected', 'false');
    selected = (selected + delta + vis.length) % vis.length;
    vis[selected].setAttribute('aria-selected', 'true');
    vis[selected].scrollIntoView({ block: 'nearest' });
  }

  function activate() {
    const vis = visibleItems();
    const sel = vis[selected];
    if (!sel?.dataset.href) return;
    if (sel.dataset.external === 'true') {
      window.open(sel.dataset.href, '_blank', 'noopener,noreferrer');
    } else {
      window.location.assign(sel.dataset.href);
    }
  }

  // Hotkeys
  document.addEventListener('keydown', (e) => {
    const isMod = e.metaKey || e.ctrlKey;
    if (isMod && e.key.toLowerCase() === 'k') { e.preventDefault(); open(); return; }
    if (el.dataset.open !== 'true') return;
    if (e.key === 'Escape') { e.preventDefault(); close(); }
    else if (e.key === 'ArrowDown') { e.preventDefault(); move(1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); move(-1); }
    else if (e.key === 'Enter') { e.preventDefault(); activate(); }
  });

  // Click triggers
  document.addEventListener('click', (e) => {
    const t = e.target.closest('[data-action]');
    if (!t) return;
    if (t.dataset.action === 'palette-open') { e.preventDefault(); open(); }
    if (t.dataset.action === 'palette-close') { e.preventDefault(); close(); }
  });

  // Item click → navigate
  list.addEventListener('click', (e) => {
    const item = e.target.closest('.palette__item');
    if (!item?.dataset.href) return;
    if (item.dataset.external === 'true') {
      window.open(item.dataset.href, '_blank', 'noopener,noreferrer');
    } else {
      window.location.assign(item.dataset.href);
    }
  });

  input.addEventListener('input', (e) => apply(e.target.value));
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

#[cfg(test)]
mod tests {
    #[test]
    fn css_bundle_includes_all_layers() {
        let s = super::css_bundle();
        assert!(s.contains("--primary-color"), "tokens layer missing");
        assert!(s.contains("box-sizing"), "base layer missing");
        assert!(
            s.contains(".btn") || s.contains(".button"),
            "components layer missing"
        );
        assert!(s.contains(".app-layout"), "layout layer missing");
    }

    #[test]
    fn tokens_include_new_scale() {
        let s = super::css_bundle();
        for tok in [
            "--text-base",
            "--text-2xl",
            "--space-2xl",
            "--surface-1",
            "--primary-button",
            "--focus-ring",
        ] {
            assert!(s.contains(tok), "missing token: {tok}");
        }
    }

    #[test]
    fn palette_js_present_and_self_invoking() {
        let js = super::palette_js();
        assert!(js.contains("cmdk"));
        assert!(js.contains("Meta+K") || js.contains("metaKey"));
        assert!(js.starts_with("\n(function") || js.contains("(function "));
    }
}
