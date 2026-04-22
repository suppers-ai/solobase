//! htmx fragment returned by `POST /auth/cli/issue`.
//!
//! Swapped into `#code-panel` on the CLI-login page. Intentionally emits
//! no `<html>`/`<body>` wrapper — it is spliced into an existing page by
//! htmx.

use maud::{html, Markup};

use crate::blocks::auth::view_models::CliCodeFragmentViewModel;

pub fn render(vm: &CliCodeFragmentViewModel) -> Markup {
    html! {
        p {
            "Copy this code into your terminal. It expires in "
            (vm.expires_in_minutes) " minutes."
        }
        div class="pat-code" id="cli-code-value" { (vm.code) }
        button
            type="button"
            onclick="navigator.clipboard.writeText(document.getElementById('cli-code-value').innerText)"
        { "Copy" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_code_and_expiry() {
        let vm = CliCodeFragmentViewModel {
            code: "abc123xyz".into(),
            expires_in_minutes: 15,
        };
        let out = render(&vm).into_string();
        assert!(out.contains("abc123xyz"));
        assert!(out.contains("15 minutes"));
        assert!(out.contains("Copy"));
        assert!(out.contains(r#"class="pat-code""#));
        // No <html>/<body> — this is a fragment.
        assert!(!out.contains("<html"));
        assert!(!out.contains("<body"));
    }
}
