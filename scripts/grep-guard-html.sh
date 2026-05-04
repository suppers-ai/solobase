#!/usr/bin/env bash
# Phase 5c grep-guard: full-page HTML must originate from solobase-core/src/ui/.
# Catches blocks shipping their own page chrome instead of using a template.
#
# Signals matched (in .rs source):
#   - Maud's (DOCTYPE html) single-token compact form.
#   - Maud's (DOCTYPE) two-token form (followed by `html lang=...`).
#   - Raw "<html" or "<!DOCTYPE" string literals.
#
# Exemptions (extend with care):
#   - crates/solobase-core/src/blocks/legalpages/mod.rs — the public legal-page
#     renderer (`/b/legalpages/{terms,privacy}`) intentionally ships its own
#     chrome (different audience, different typography, deployment-configured
#     branding) and pre-dates the design system. Phase 5d / follow-up will
#     introduce a public_page template and remove this exemption.
set -euo pipefail
hits=$(grep -rlnE --include='*.rs' \
  '\(DOCTYPE\s+html\)|\(DOCTYPE\)|<!DOCTYPE|<html\b' \
  crates/ \
  | grep -vE '^crates/solobase-core/src/ui/|^crates/solobase-core/src/blocks/legalpages/mod\.rs$' \
  || true) # grep exits 1 on empty input under pipefail; || true normalises that
if [ -n "$hits" ]; then
  echo "ERROR: full-page HTML markers found outside crates/solobase-core/src/ui/:" >&2
  echo "$hits" >&2
  echo "" >&2
  echo "Page-level HTML must come from a template in solobase-core/src/ui/templates.rs" >&2
  echo "or solobase-core/src/ui/layout.rs. If this is intentional, update the guard." >&2
  exit 1
fi
echo "OK: no full-page HTML markers outside crates/solobase-core/src/ui/."
