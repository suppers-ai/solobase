#!/usr/bin/env bash
# test-flow.sh — End-to-end test of the Solobase API flow.
# Requires the local dev stack to be running (bash scripts/local-dev.sh).
set -euo pipefail

CLOUD=http://localhost:8080
API_SECRET=dev-secret
COOKIES=$(mktemp)
PASS=0
FAIL=0

check() {
    local name="$1"
    local expected="$2"
    local actual="$3"
    if echo "$actual" | grep -q "$expected"; then
        echo "  PASS: $name"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $name (expected '$expected' in response)"
        echo "        got: $actual"
        FAIL=$((FAIL + 1))
    fi
}

check_status() {
    local name="$1"
    local expected="$2"
    local actual="$3"
    if [ "$actual" = "$expected" ]; then
        echo "  PASS: $name"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $name (expected status $expected, got $actual)"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== Solobase E2E Test Flow ==="
echo ""

# 1. List plans
echo "[1] GET /api/plans"
RESP=$(curl -sf "$CLOUD/api/plans")
check "returns plans" "free" "$RESP"
check "has hobby plan" "hobby" "$RESP"
check "has business plan" "business" "$RESP"
PLAN_COUNT=$(echo "$RESP" | python3 -c "import sys,json; print(len(json.load(sys.stdin)))" 2>/dev/null || echo "?")
if [ "$PLAN_COUNT" = "5" ]; then
    echo "  PASS: 5 plans returned"
    PASS=$((PASS + 1))
else
    echo "  FAIL: expected 5 plans, got $PLAN_COUNT"
    FAIL=$((FAIL + 1))
fi

# 2. Check admin nodes
echo ""
echo "[2] GET /api/admin/nodes"
HTTP_CODE=$(curl -so /dev/null -w "%{http_code}" "$CLOUD/api/admin/nodes" \
    -H "Authorization: Bearer $API_SECRET")
check_status "admin nodes accessible" "200" "$HTTP_CODE"

RESP=$(curl -sf "$CLOUD/api/admin/nodes" -H "Authorization: Bearer $API_SECRET")
check "local-dev node registered" "local-dev" "$RESP"

# 3. Verify admin auth is enforced
echo ""
echo "[3] Admin auth enforcement"
HTTP_CODE=$(curl -so /dev/null -w "%{http_code}" "$CLOUD/api/admin/nodes")
check_status "admin without auth returns 403" "403" "$HTTP_CODE"

# 4. Create dev session
echo ""
echo "[4] POST /api/dev/session"
RESP=$(curl -sf -c "$COOKIES" "$CLOUD/api/dev/session")
check "session created" "dev session created" "$RESP"
check "dev user email" "dev@localhost" "$RESP"

# 5. Check /api/me
echo ""
echo "[5] GET /api/me"
RESP=$(curl -sf -b "$COOKIES" "$CLOUD/api/me")
check "current user" "Local Dev" "$RESP"

# 6. Create tenant
echo ""
echo "[6] POST /api/tenants (create)"
RESP=$(curl -sf -b "$COOKIES" -X POST "$CLOUD/api/tenants" \
    -H "Content-Type: application/json" \
    -d '{"subdomain":"test-app","plan":"hobby"}')
check "tenant created" "test-app" "$RESP"
check "tenant running" "running" "$RESP"
TENANT_ID=$(echo "$RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])" 2>/dev/null || echo "")
if [ -z "$TENANT_ID" ]; then
    echo "  FAIL: could not extract tenant ID"
    FAIL=$((FAIL + 1))
else
    echo "  PASS: tenant ID = $TENANT_ID"
    PASS=$((PASS + 1))
fi

# 7. List tenants
echo ""
echo "[7] GET /api/tenants (list)"
RESP=$(curl -sf -b "$COOKIES" "$CLOUD/api/tenants")
check "tenant in list" "test-app" "$RESP"

# 8. Get tenant detail
echo ""
echo "[8] GET /api/tenants/{id}"
RESP=$(curl -sf -b "$COOKIES" "$CLOUD/api/tenants/$TENANT_ID")
check "tenant detail" "test-app" "$RESP"
check "plan is hobby" "hobby" "$RESP"

# 9. Pause tenant
echo ""
echo "[9] POST /api/tenants/{id}/pause"
HTTP_CODE=$(curl -so /dev/null -w "%{http_code}" -b "$COOKIES" \
    -X POST "$CLOUD/api/tenants/$TENANT_ID/pause")
check_status "pause returns 200" "200" "$HTTP_CODE"

# 10. Resume tenant
echo ""
echo "[10] POST /api/tenants/{id}/resume"
HTTP_CODE=$(curl -so /dev/null -w "%{http_code}" -b "$COOKIES" \
    -X POST "$CLOUD/api/tenants/$TENANT_ID/resume")
check_status "resume returns 200" "$HTTP_CODE" "200"

# 11. Delete tenant
echo ""
echo "[11] DELETE /api/tenants/{id}"
HTTP_CODE=$(curl -so /dev/null -w "%{http_code}" -b "$COOKIES" \
    -X DELETE "$CLOUD/api/tenants/$TENANT_ID")
check_status "delete returns 204" "204" "$HTTP_CODE"

# 12. Verify deletion
echo ""
echo "[12] GET /api/tenants (verify empty)"
RESP=$(curl -sf -b "$COOKIES" "$CLOUD/api/tenants")
if echo "$RESP" | grep -q "test-app"; then
    echo "  FAIL: tenant still present after delete"
    FAIL=$((FAIL + 1))
else
    echo "  PASS: tenant gone after delete"
    PASS=$((PASS + 1))
fi

# 13. Verify unauthenticated access blocked
echo ""
echo "[13] Auth enforcement"
HTTP_CODE=$(curl -so /dev/null -w "%{http_code}" "$CLOUD/api/tenants")
check_status "tenants without auth returns 401" "401" "$HTTP_CODE"

HTTP_CODE=$(curl -so /dev/null -w "%{http_code}" "$CLOUD/api/me")
check_status "me without auth returns 401" "401" "$HTTP_CODE"

# 14. Mock node health
echo ""
echo "[14] GET mock-node /api/health"
RESP=$(curl -sf http://localhost:9090/api/health)
check "node healthy" "ok" "$RESP"

# Summary
echo ""
echo "=== Results ==="
echo "  Passed: $PASS"
echo "  Failed: $FAIL"
TOTAL=$((PASS + FAIL))
echo "  Total:  $TOTAL"
echo ""

rm -f "$COOKIES"

if [ "$FAIL" -gt 0 ]; then
    echo "SOME TESTS FAILED"
    exit 1
else
    echo "ALL TESTS PASSED"
    exit 0
fi
