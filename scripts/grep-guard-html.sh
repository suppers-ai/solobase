#!/usr/bin/env bash
# Phase 5c grep-guard: full-page HTML must originate from solobase-core/src/ui/.
# Catches blocks shipping their own page chrome instead of using a template.
#
# Signals matched (in .rs source):
#   - Maud's (DOCTYPE html) token inside an html! macro.
#   - Raw "<html" or "<!DOCTYPE" string literals.
set -euo pipefail
hits=$(grep -rlnE --include='*.rs' \
  '\(DOCTYPE\s+html\)|<!DOCTYPE|<html\b' \
  crates/ \
  | grep -v '^crates/solobase-core/src/ui/' || true)
if [ -n "$hits" ]; then
  echo "ERROR: full-page HTML markers found outside crates/solobase-core/src/ui/:" >&2
  echo "$hits" >&2
  echo "" >&2
  echo "Page-level HTML must come from a template in solobase-core/src/ui/templates.rs" >&2
  echo "or solobase-core/src/ui/layout.rs. If this is intentional, update the guard." >&2
  exit 1
fi
echo "OK: no full-page HTML markers outside crates/solobase-core/src/ui/."
