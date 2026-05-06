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

const LLM_CHAT_JS: &str = include_str!("assets/llm-chat.js");

/// Embedded vanilla-JS bundle for the LLM chat surface — markdown, message
/// rendering, model management, chat submission, thread creation/selection.
/// Consumed by the unified LLM page handler and (for the conversation lens)
/// by the Messages context_detail handler.
pub fn llm_chat_js() -> &'static str {
    LLM_CHAT_JS
}

/// LLM chat JS URL with content hash, e.g. `/b/static/llm-chat-a1b2c3d4.js`.
/// Not minified — readability matters for a script that's debugged in
/// Chrome devtools.
pub fn llm_chat_js_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| {
        format!(
            "/b/static/llm-chat-{}.js",
            short_hash(llm_chat_js().as_bytes())
        )
    })
}

const FILES_BROWSER_JS: &str = include_str!("assets/files-browser.js");

/// Embedded vanilla-JS bundle for the file-browser surfaces — drag-drop
/// upload, bulk select, kebab menus, share modal, upload modal,
/// confirm-delete. Consumed by `pages_user::object_list_page` and
/// `cloudstorage_page`.
pub fn files_browser_js() -> &'static str {
    FILES_BROWSER_JS
}

/// Files-browser JS URL with content hash, e.g. `/b/static/files-browser-a1b2c3d4.js`.
pub fn files_browser_js_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| {
        format!(
            "/b/static/files-browser-{}.js",
            short_hash(files_browser_js().as_bytes())
        )
    })
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

/// Vanilla JS for the mobile sidebar drawer. Toggles `body[data-drawer-open]`
/// from clicks on `[data-action="drawer-open"]` (the hamburger), the overlay
/// (`[data-action="drawer-close"]`), Escape, or any sidebar nav-link click
/// (so navigation auto-collapses the drawer).
pub fn drawer_js() -> &'static str {
    r#"
(function () {
  if (window.__drawerInit) return;
  window.__drawerInit = true;
  var body = document.body;
  function open() { body.setAttribute('data-drawer-open', 'true'); }
  function close() { body.removeAttribute('data-drawer-open'); }
  document.addEventListener('click', function (e) {
    var t = e.target;
    if (!(t instanceof Element)) return;
    var actEl = t.closest('[data-action]');
    var action = actEl ? actEl.getAttribute('data-action') : null;
    if (action === 'drawer-open') { open(); e.preventDefault(); return; }
    if (action === 'drawer-close') { close(); e.preventDefault(); return; }
    if (body.hasAttribute('data-drawer-open') && t.closest('.sidebar a')) {
      close();
    }
  });
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape' && body.hasAttribute('data-drawer-open')) {
      close();
    }
  });
})();
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
        assert!(s.contains(".shell"), "layout layer missing");
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

    #[test]
    fn drawer_js_handles_open_close_esc_and_navlink() {
        let js = super::drawer_js();
        assert!(js.contains("'drawer-open'"));
        assert!(js.contains("'drawer-close'"));
        assert!(js.contains("'Escape'"));
        assert!(js.contains(".sidebar a"));
        assert!(js.contains("data-drawer-open"));
        // Self-invoking + idempotent guard.
        assert!(js.contains("__drawerInit"));
    }

    #[test]
    fn llm_chat_js_is_self_invoking_and_exposes_init() {
        let js = super::llm_chat_js();
        assert!(js.contains("(function ()") || js.contains("(function()"));
        assert!(js.contains("__solobaseLlmChatLoaded"));
        assert!(js.contains("window.solobaseLlmChat = { init: init }"));
        for sym in [
            "handleChatSubmit",
            "createNewThread",
            "selectThread",
            "onModelChange",
            "unloadLocalModel",
        ] {
            assert!(
                js.contains(&format!("window.{sym} = {sym}")),
                "missing global re-export for {sym}"
            );
        }
    }

    #[test]
    fn files_browser_js_url_has_content_hash() {
        let url = super::files_browser_js_url();
        assert!(url.starts_with("/b/static/files-browser-"));
        assert!(url.ends_with(".js"));
        let hash = url
            .trim_start_matches("/b/static/files-browser-")
            .trim_end_matches(".js");
        assert_eq!(hash.len(), 8);
    }

    #[test]
    fn llm_chat_js_url_has_content_hash() {
        let url = super::llm_chat_js_url();
        assert!(url.starts_with("/b/static/llm-chat-"));
        assert!(url.ends_with(".js"));
        assert!(
            !url.ends_with(".min.js"),
            "we deliberately ship un-minified"
        );
        let mid = url
            .trim_start_matches("/b/static/llm-chat-")
            .trim_end_matches(".js");
        assert_eq!(mid.len(), 8, "expected 8-char short hash, got: {mid}");
        assert!(mid.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
