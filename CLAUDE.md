# Development Guidelines

- Always fix the real issue. No code smells, no shortcuts, no workarounds.
- If the right fix requires touching many files, touch many files.
- No sync bridges (`poll_once`, `block_on`) to avoid propagating async. If something is async, callers must be async.
- No magic code or implicit mapping layers. Keep things explicit and easy to maintain. If a value has a prefix, it has the same prefix everywhere (env vars, D1, config API). No translation between representations.
- Config variable naming: `SOLOBASE_SHARED__*` = shared app config (any block reads, admin writes). `{ORG}__{BLOCK}__*` = block-scoped (only owner + admin). `SOLOBASE_*` (no `__`) = infrastructure, never in DB.
- Blocks declare their own config vars via `ConfigVar` in `BlockInfo::config_keys`. Shared vars are defined centrally in `solobase-core/src/config_vars.rs`. No hardcoded lists — validation rules derived from conventions (suffix `_SECRET`/`_KEY` = sensitive, suffix `_URL` = validated, `input_type` = UI rendering).
