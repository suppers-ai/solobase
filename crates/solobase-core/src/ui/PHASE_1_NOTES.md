# UI module — Phase 1 state (Solobase UI Cleanup)

Spec: `workspace/docs/superpowers/specs/2026-04-30-solobase-ui-cleanup-design.md`
Plan: `workspace/docs/superpowers/plans/2026-04-30-solobase-ui-cleanup-phase-1-foundations.md`

## What's available now

- Tokens (`assets/tokens.css`): `--text-xs/sm/base/lg/xl/2xl`, `--space-2xl`,
  `--surface-1/2/3`, `--primary-button`, `--focus-ring`. Old tokens still present.
- Components (`components.rs`):
  - `button(variant, size, label, extra_attrs)` — `BtnVariant`, `CtrlSize`
  - `text_input` / `textarea_input` / `select_input` — `FieldProps`
  - `card(title, body, actions)`
  - `badge(variant, label)` — `BadgeVariant`
  - `avatar(seed, size)` — deterministic FNV-32 gradient
  - `data_table(columns, rows, row_href, empty)` — `TableCol`
  - `empty_state(icon, title, body, action)`
  - `pagination(page, per_page, total, base_href)`
- Templates (`templates.rs`):
  - `list_page(header, filters, table, pagination)`
  - `detail_page(hero, sections, meta)` — `DetailHero`, `DetailMeta`
  - `form_page(header, tabs, sections, submit_url, method, save_label)` — `FormSection`
  - `dashboard_page(header, stats, primary_card, secondary_card, full_width_card)` — `StatTile`
  - `chat_page(thread_list, messages, composer, right_rail)`
  - `auth_split(brand, form_card)` — `BrandPanel`
  - `status_page(code, title, body, primary_action)`
- Shell (`shell.rs`):
  - `shell(nav_groups, user, current_path, logo, logo_icon, topbar, body)`
  - `Topbar { crumbs, primary_action, show_palette }`, `Crumb { label, href }`
  - `render_topbar` skips rendering entirely when all inputs are empty
  - `one_group(items)` — wraps a flat list for backward-compat callers
- Sidebar (`sidebar.rs`): `sidebar_grouped(groups, user, current_path, logo, logo_icon)`,
  `NavGroup { label, items }`. Old `sidebar(...)` is untouched.
- Palette (`palette.rs`): `palette(entries)`, `PaletteEntry`. JS at `assets::palette_js()`.

## What's not done yet (later phases)

- Phase 2: shell adoption — every page declares its sidebar groups, breadcrumbs,
  primary action; auth flows move to `auth_split`; `/`, 404, 403, 500 use `status_page`.
- Phase 3: admin port to templates, Settings consolidation under `/b/admin/settings/{tab}`,
  SQL first-class entry.
- Phase 4: end-user portal IA reconciliation, mobile pass.
- Phase 5: chat surfaces, Vector port, file-browser unification, dead-CSS removal,
  token renames (`--success-color` → `--accent-success`, `--danger-color` → `--accent-danger`, etc.).

## Backward compatibility notes

- `layout::block_shell` and every page calling it are **untouched**. The new
  shell engine coexists, ready for Phase 2 callers to opt into.
- Old button HTML in pages (uses pre-existing `.btn` styles) keeps rendering.
  The new canonical button class set (`btn--primary` etc.) is additive — both
  resolve.
- No public function was renamed in this phase, **with one exception**:
  `data_table`, `empty_state`, and `pagination` previously existed in
  `components.rs` with different signatures (htmx-target-based, simpler
  argument lists). The canonical names were given to the new Phase 1 API;
  the old functions were renamed to `*_v1` and their 8 call sites in
  `userportal.rs`, `products/pages.rs`, `admin/pages/users.rs`,
  `admin/pages/logs.rs`, `admin/pages/blocks.rs` were mechanically
  updated. Behavior is identical — purely a rename. Phase 3 retires `*_v1`
  callers as it ports those pages onto the new templates.

## Verification

Smoke test on 2026-04-30 against this branch (port 8093) vs `main` (port 8091):
- All probed routes return identical status codes (200/403).
- Body HTML for `/b/auth/login` is byte-identical; only the CSS bundle hash
  differs (`app-4668f8b4.css` → `app-8fb34226.css`), reflecting the new
  tokens/components/templates added to the bundle.
- 408 unit tests pass (`cargo test -p solobase-core --lib`).
