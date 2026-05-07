#!/bin/bash
#
# audit-wrap-grants.sh — static-analysis WRAP-grant coverage for solobase-core
#
# Walks every `db::{list,create,update,delete,count,get,find_one}` callsite
# in `crates/solobase-core/src/blocks/`, derives the calling block from the
# file path and the table-owning block from the table's `{org}__{block}__`
# prefix, and verifies the owning block declares a `ResourceGrant` covering
# the call.
#
# Background: WRAP enforces cross-block table access at runtime, but only
# when the calling site routes through the typed `db::*` client AND the
# owning block's `BlockInfo::grants` contains a matching `ResourceGrant`.
# Render-function unit tests don't exercise the call path, so missing
# grants ship to main green and only surface as 500s in production.
#
# This script catches the static gap in CI before the bug ships. Two such
# gaps were found at PR-time in May 2026 (PR #75 + PR #77) — both took
# multiple commits to land because the failure was discovered in CI not
# code review.
#
# Out of scope:
#   - Raw SQL paths (`db::query_raw`, `db::exec_raw`) — admin-only by design.
#   - HTTP-style cross-block calls (`ctx.call_block_buffered(...)`) — they
#     don't set `wrap.resource` meta, so WRAP doesn't gate them today.
#     That's a separate design question; see PR #81 description.
#   - Non-Database grants (`Network`, `Storage` resource types) — the script
#     only checks Database access. Those grant types follow different rules.
#   - Tables outside the `{org}__{block}__` convention (none currently exist
#     in solobase-core but flagged if found).

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

BLOCKS_DIR="crates/solobase-core/src/blocks"

if [ ! -d "$BLOCKS_DIR" ]; then
  echo "::error::$BLOCKS_DIR not found — run from a solobase repo root."
  exit 2
fi

# ---------- Phase 1: collect all string constants ----------
# Two const shapes used in solobase-core:
#   pub const TABLE: &str = "suppers_ai__auth__users";
#   const FOO_COLLECTION: &str = "suppers_ai__foo__bar";
#   pub const AUTH_BLOCK_ID: &str = "suppers-ai/auth";
#
# We index ALL of them globally by bare name. When a callsite refers to
# `module::TABLE`, we keep only the trailing identifier (`TABLE`) — the
# global map is keyed by the bare name. Collisions are rare in practice
# (only `TABLE` collides across auth/repo/* modules, but they all have the
# same convention so we record per-(file, name) and look up with a
# file-scope-aware fallback for `TABLE`).

declare -A CONST_VALUE          # bare_name -> value (latest seen — global fallback only)
declare -A FILE_CONST_VALUE     # "${file}::${bare_name}" -> value (per-file definitions)
declare -A FILE_CONST_NAMES     # file -> space-separated list of locally-defined names
declare -A FILE_USE_ALIAS       # "${file}::${alias}" -> source_name (from `use ... as alias`)
declare -A SIBLING_CONST        # "${dir}::${name}" -> value (for `super::NAME` lookups)

while IFS= read -r line; do
  # Format: file:lineno:    pub const NAME: &str = "VALUE";
  file="${line%%:*}"
  rest="${line#*:}"
  rest="${rest#*:}"
  re_const='const[[:space:]]+([A-Z_]+)[[:space:]]*:[[:space:]]*&str[[:space:]]*=[[:space:]]*"([^"]+)"'
  if [[ "$rest" =~ $re_const ]]; then
    name="${BASH_REMATCH[1]}"
    value="${BASH_REMATCH[2]}"
    CONST_VALUE["$name"]="$value"
    FILE_CONST_VALUE["${file}::${name}"]="$value"
    FILE_CONST_NAMES["$file"]="${FILE_CONST_NAMES[$file]:-} $name"
    # Index the const at the directory level so siblings can look it up via `super::NAME`.
    dir="$(dirname "$file")"
    SIBLING_CONST["${dir}::${name}"]="$value"
  fi
# Anchor at start-of-line so we skip function-scoped constants like
#   `        const PROVIDERS_COLLECTION = "..."`
# inside a fn body (typically migration helpers). Module-level consts always
# start at column 0 in this codebase. Visibility modifier may be `pub`,
# `pub(crate)`, `pub(super)`, etc.
done < <(grep -rEn "^(pub(\([^)]+\))?[[:space:]]+)?const [A-Z_]+: &str = \"[^\"]+\"" "$BLOCKS_DIR" 2>/dev/null || true)

# Parse `use ... as` aliases. Both crate-rooted and super-relative paths
# matter — many files import `use crate::blocks::auth::USERS_COLLECTION as USERS`.
# We don't model module scope precisely; we just record alias → source-bare-name
# and rely on the source name being globally unique (most `_COLLECTION`
# constants are; `COLLECTION` and `TABLE` are not — that's the case the
# alias map specifically resolves).
#
# Forms covered:
#   use super::FOO as BAR;
#   use super::{FOO as BAR, BAZ};
#   use crate::blocks::PATH::FOO as BAR;
#   use crate::blocks::PATH::{FOO as BAR, BAZ as QUX};
#   use self::PATH::{FOO as BAR};

re_use_simple_as='use[[:space:]]+(super|self|crate)::[A-Za-z_:]+::([A-Z_]+)[[:space:]]+as[[:space:]]+([A-Z_]+)'
re_use_super_simple='use[[:space:]]+super::([A-Z_]+)[[:space:]]+as[[:space:]]+([A-Z_]+)'
re_use_brace_item='([A-Z_]+)[[:space:]]+as[[:space:]]+([A-Z_]+)'

while IFS= read -r line; do
  file="${line%%:*}"
  rest="${line#*:}"
  rest="${rest#*:}"
  # Simple `use ROOT::PATH::NAME as ALIAS;`
  if [[ "$rest" =~ $re_use_simple_as ]]; then
    src="${BASH_REMATCH[2]}"
    alias="${BASH_REMATCH[3]}"
    FILE_USE_ALIAS["${file}::${alias}"]="$src"
    continue
  fi
  # `use super::NAME as ALIAS;` (no further path segments)
  if [[ "$rest" =~ $re_use_super_simple ]]; then
    src="${BASH_REMATCH[1]}"
    alias="${BASH_REMATCH[2]}"
    FILE_USE_ALIAS["${file}::${alias}"]="$src"
    continue
  fi
  # Brace form: pull out every `X as Y` substring inside the outermost `{...}`.
  if [[ "$rest" == *"{"*"as"*"}"* ]]; then
    inner="${rest#*\{}"
    inner="${inner%%\}*}"
    IFS=',' read -ra items <<< "$inner"
    for item in "${items[@]}"; do
      item="$(echo "$item" | xargs)"  # trim
      if [[ "$item" =~ $re_use_brace_item ]]; then
        src="${BASH_REMATCH[1]}"
        alias="${BASH_REMATCH[2]}"
        FILE_USE_ALIAS["${file}::${alias}"]="$src"
      fi
    done
  fi
done < <(grep -rEn "^use[[:space:]]" "$BLOCKS_DIR" 2>/dev/null || true)

# Catch nested-brace `use crate::{ blocks::{ auth::{ FOO_COLLECTION as FOO } } };`
# patterns that span multiple lines. The grep above matches only the `use ` line
# itself, missing the alias on a continuation line. The pattern `\bX as Y\b`
# where both X and Y are SCREAMING_SNAKE is unambiguous in this codebase.
# Use grep -oE per-file to extract every match (a single line can contain
# multiple alias pairs comma-separated inside one brace import).
while IFS= read -r file; do
  while IFS= read -r match; do
    [ -z "$match" ] && continue
    if [[ "$match" =~ ^([A-Z_]{4,})[[:space:]]+as[[:space:]]+([A-Z_]{2,})$ ]]; then
      src="${BASH_REMATCH[1]}"
      alias="${BASH_REMATCH[2]}"
      if [ -n "${CONST_VALUE[$src]:-}" ]; then
        FILE_USE_ALIAS["${file}::${alias}"]="$src"
      fi
    fi
  done < <(grep -oE "[A-Z_]{4,}[[:space:]]+as[[:space:]]+[A-Z_]{2,}" "$file" 2>/dev/null || true)
done < <(find "$BLOCKS_DIR" -name '*.rs' 2>/dev/null)

# ---------- Phase 2: collect grants per-owning-block ----------
# Pattern:  ResourceGrant::{read,read_write}(GRANTEE, RESOURCE)[.typed(TYPE)]
# Grants live in a block's `BlockInfo::grants(vec![...])` — we attribute the
# grant to the file's owning block (the directory or .rs filename under
# blocks/).
#
# Each grant entry is encoded as:
#   "${owner_block_id}|${grantee}|${resource}|${type}"
# where TYPE is "Database" by default or whatever appears after .typed().

GRANTS=()

# Resolve a token (constant name, possibly-qualified path, or string literal)
# to its actual table-name string. File-aware: tokens are resolved against
# the file's local consts and `use ... as` aliases first, then the global
# CONST_VALUE map as last resort.
#
# Examples (from a callsite in `blocks/admin/logs.rs`):
#   "literal_name"            -> literal_name
#   COLLECTION                -> resolved via file-local + use-alias chain
#   crate::FOO::BAR           -> bare BAR via global fallback
#   super::USERS_COLLECTION   -> bare USERS_COLLECTION via parent-dir SIBLING_CONST
resolve_token() {
  local tok="$1"
  local file="$2"
  if [[ "$tok" =~ ^\"(.+)\"$ ]]; then
    echo "${BASH_REMATCH[1]}"
    return
  fi
  # Stripped bare identifier (drop `module::path::` prefix).
  local bare="${tok##*::}"
  # 1. Per-file definition.
  if [ -n "${FILE_CONST_VALUE[${file}::${bare}]:-}" ]; then
    echo "${FILE_CONST_VALUE[${file}::${bare}]}"
    return
  fi
  # 2. Per-file `use ... as` alias — chase to the source name. Look in
  #    sibling modules first, then fall through to the global map.
  if [ -n "${FILE_USE_ALIAS[${file}::${bare}]:-}" ]; then
    local src="${FILE_USE_ALIAS[${file}::${bare}]}"
    local parent_dir="$(dirname "$file")"
    if [ -n "${SIBLING_CONST[${parent_dir}::${src}]:-}" ]; then
      echo "${SIBLING_CONST[${parent_dir}::${src}]}"
      return
    fi
    if [ -n "${CONST_VALUE[$src]:-}" ]; then
      echo "${CONST_VALUE[$src]}"
      return
    fi
  fi
  # 3. `super::NAME` reference (no rename) — look in the parent dir.
  if [[ "$tok" =~ ^super:: ]]; then
    local parent_dir="$(dirname "$file")"
    if [ -n "${SIBLING_CONST[${parent_dir}::${bare}]:-}" ]; then
      echo "${SIBLING_CONST[${parent_dir}::${bare}]}"
      return
    fi
  fi
  # 4. Global fallback — only safe if the bare name is unambiguous across
  #    the codebase. Used for grant declarations (top-level scope) where
  #    file-aware lookup isn't a fit.
  if [ -n "${CONST_VALUE[$bare]:-}" ]; then
    echo "${CONST_VALUE[$bare]}"
    return
  fi
  echo "<unresolved:$tok>"
}

# Convert a file path like crates/solobase-core/src/blocks/files/mod.rs or
# crates/solobase-core/src/blocks/network.rs into the block id
# `suppers-ai/{name}`.
file_to_block_id() {
  local path="$1"
  # Strip the prefix to get blocks/<rest>
  local rel="${path#$BLOCKS_DIR/}"
  # Take the first path segment, stripping trailing .rs
  local first="${rel%%/*}"
  first="${first%.rs}"
  echo "suppers-ai/${first//_/-}"
}

# Convert a table name (e.g. suppers_ai__auth__sessions) to its owner block
# id (suppers-ai/auth). Returns empty string if the name doesn't follow
# the {org}__{block}__{rest} convention.
table_to_owner() {
  local table="$1"
  if [[ "$table" =~ ^([a-z0-9_]+)__([a-z0-9_]+)__ ]]; then
    local org="${BASH_REMATCH[1]//_/-}"
    local block="${BASH_REMATCH[2]//_/-}"
    echo "${org}/${block}"
    return
  fi
  echo ""
}

while IFS= read -r line; do
  file="${line%%:*}"
  rest="${line#*:}"
  rest="${rest#*:}"
  # Match: ResourceGrant::read("a", "b")  or  ResourceGrant::read_write(IDENT, IDENT)
  # The args may be string literals or constant identifiers (with optional `super::module::` qualifier).
  # Bash requires the regex stored in a variable when it contains parens.
  re_grant='ResourceGrant::(read|read_write)\(([^,]+),[[:space:]]*([^)]+)\)'
  if [[ "$rest" =~ $re_grant ]]; then
    kind="${BASH_REMATCH[1]}"
    grantee_raw="${BASH_REMATCH[2]// /}"
    resource_raw="${BASH_REMATCH[3]// /}"
    grantee="$(resolve_token "$grantee_raw" "$file")"
    resource="$(resolve_token "$resource_raw" "$file")"
    # Grant type: default Database; if the same line has .typed(...), pick that
    type="Database"
    re_typed='\.typed\(([^)]*ResourceType::)?([A-Za-z]+)\)'
    if [[ "$rest" =~ $re_typed ]]; then
      type="${BASH_REMATCH[2]}"
    fi
    # Owning block = the block this file lives in
    owner="$(file_to_block_id "$file")"
    GRANTS+=("${owner}|${grantee}|${resource}|${type}|${kind}")
  fi
done < <(grep -rEn "ResourceGrant::(read|read_write)\(" "$BLOCKS_DIR" 2>/dev/null || true)

# ---------- Phase 3: walk db::* callsites and check coverage ----------

# Returns "OK" if a grant covers (caller, table); otherwise "MISSING".
# Grant matches when:
#   - resource_type is Database (or the grant's type is empty/wildcard)
#   - grantee == caller OR grantee == "*"
#   - resource == table OR (resource ends with "*" AND table starts with the prefix)
check_coverage() {
  local caller="$1" table="$2"
  local owner
  owner="$(table_to_owner "$table")"
  if [ -z "$owner" ]; then
    echo "NON_CONVENTIONAL"
    return
  fi
  if [ "$caller" = "$owner" ]; then
    echo "OWN"
    return
  fi
  for g in "${GRANTS[@]}"; do
    IFS='|' read -r g_owner g_grantee g_resource g_type _g_kind <<< "$g"
    [ "$g_owner" != "$owner" ] && continue
    [ "$g_type" != "Database" ] && continue
    if [ "$g_grantee" != "*" ] && [ "$g_grantee" != "$caller" ]; then
      continue
    fi
    if [ "$g_resource" = "$table" ] || [ "$g_resource" = "*" ]; then
      echo "OK"; return
    fi
    # Prefix match: grant resource ends with `*`
    if [[ "$g_resource" == *\* ]]; then
      local prefix="${g_resource%\*}"
      if [[ "$table" == ${prefix}* ]]; then
        echo "OK"; return
      fi
    fi
  done
  echo "MISSING"
}

declare -i total=0 missing=0 unresolved=0 nonconv=0
declare -A SEEN_PAIRS
MISSING_LINES=()
UNRESOLVED_LINES=()
NONCONV_LINES=()

while IFS= read -r line; do
  file="${line%%:*}"
  rest="${line#*:}"
  lineno="${rest%%:*}"
  rest="${rest#*:}"
  # Match db::list(ctx, COLLECTION, ...) — second arg is the table.
  # Permit an optional `&` prefix on the arg. Pattern in a variable for bash regex.
  re_dbcall='db::(list|create|update|delete|count|get|find_one)[[:space:]]*\([[:space:]]*ctx[[:space:]]*,[[:space:]]*&?([A-Za-z_:]+|"[^"]+")[[:space:]]*[,)]'
  if [[ "$rest" =~ $re_dbcall ]]; then
    arg="${BASH_REMATCH[2]}"
    table="$(resolve_token "$arg" "$file")"
    caller="$(file_to_block_id "$file")"
    pair_key="${caller}|${table}"
    [ -n "${SEEN_PAIRS[$pair_key]:-}" ] && continue
    SEEN_PAIRS["$pair_key"]=1
    total=$((total + 1))
    if [[ "$table" == "<unresolved:"* ]]; then
      unresolved=$((unresolved + 1))
      UNRESOLVED_LINES+=("${file}:${lineno}: ${caller} → ${table}")
      continue
    fi
    result="$(check_coverage "$caller" "$table")"
    case "$result" in
      OK|OWN) ;;
      MISSING)
        missing=$((missing + 1))
        owner="$(table_to_owner "$table")"
        MISSING_LINES+=("${file}:${lineno}: ${caller} → ${table} (owned by ${owner})")
        ;;
      NON_CONVENTIONAL)
        nonconv=$((nonconv + 1))
        NONCONV_LINES+=("${file}:${lineno}: ${caller} → ${table}")
        ;;
    esac
  fi
done < <(grep -rEn "db::(list|create|update|delete|count|get|find_one)\(" "$BLOCKS_DIR" 2>/dev/null || true)

# ---------- Phase 4: report ----------

echo
echo "WRAP grant audit — $(date)"
echo
echo "Indexed: ${#CONST_VALUE[@]} constants, ${#GRANTS[@]} grant decls,"
echo "         ${total} unique (caller, table) pairs across db::* callsites."
echo

if [ "${#MISSING_LINES[@]}" -gt 0 ]; then
  echo "MISSING grants (${missing}):"
  printf '  %s\n' "${MISSING_LINES[@]}"
  echo
fi

if [ "${#UNRESOLVED_LINES[@]}" -gt 0 ]; then
  echo "UNRESOLVED constants (${unresolved}) — needs human review:"
  printf '  %s\n' "${UNRESOLVED_LINES[@]}"
  echo
fi

if [ "${#NONCONV_LINES[@]}" -gt 0 ]; then
  echo "NON-CONVENTIONAL tables (${nonconv}) — owner cannot be derived from name:"
  printf '  %s\n' "${NONCONV_LINES[@]}"
  echo
fi

if [ "$missing" -gt 0 ]; then
  echo "::error::WRAP grant audit found ${missing} missing grant(s)."
  exit 1
fi

echo "OK — no missing WRAP grants."
