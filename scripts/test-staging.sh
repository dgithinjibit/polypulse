#!/bin/bash
set -e

# PolyPulse Staging Testing Script
# This script runs basic smoke tests against the staging environment

echo "🧪 Testing PolyPulse Staging Environment..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

BACKEND_URL="${BACKEND_URL:-http://localhost:8000}"
FRONTEND_URL="${FRONTEND_URL:-http://localhost:5173}"

test_count=0
pass_count=0
fail_count=0

# Function to run a test
run_test() {
    local test_name="$1"
    local test_command="$2"
    
    test_count=$((test_count + 1))
    echo -n "Testing: $test_name... "
    
    if eval "$test_command" >/dev/null 2>&1; then
        echo -e "${GREEN}✓ PASS${NC}"
        pass_count=$((pass_count + 1))
        return 0
    else
        echo -e "${RED}✗ FAIL${NC}"
        fail_count=$((fail_count + 1))
        return 1
    fi
}

echo ""
echo "🔍 Running smoke tests..."
echo ""

# Backend tests
echo "Backend Tests ($BACKEND_URL):"
run_test "Backend health endpoint" "curl -f -s $BACKEND_URL/health"
run_test "Backend responds with JSON" "curl -s $BACKEND_URL/health | grep -q 'status'"
run_test "Backend CORS headers" "curl -s -I $BACKEND_URL/health | grep -i 'access-control-allow-origin'"

# Frontend tests
echo ""
echo "Frontend Tests ($FRONTEND_URL):"
run_test "Frontend is accessible" "curl -f -s $FRONTEND_URL"
run_test "Frontend serves HTML" "curl -s $FRONTEND_URL | grep -q '<html'"
run_test "Frontend includes app root" "curl -s $FRONTEND_URL | grep -q 'id=\"root\"'"

# Database connectivity test
echo ""
echo "Database Tests:"
run_test "Database is accessible" "docker compose -f docker-compose.staging.yml exec -T db pg_isready -U polypulse -d polypulse_staging"

# Redis connectivity test
echo ""
echo "Redis Tests:"
run_test "Redis is accessible" "docker compose -f docker-compose.staging.yml exec -T redis redis-cli ping | grep -q PONG"

# Container health tests
echo ""
echo "Container Health:"
run_test "Backend container is running" "docker compose -f docker-compose.staging.yml ps backend | grep -q 'Up'"
run_test "Frontend container is running" "docker compose -f docker-compose.staging.yml ps frontend | grep -q 'Up'"
run_test "Database container is running" "docker compose -f docker-compose.staging.yml ps db | grep -q 'Up'"
run_test "Redis container is running" "docker compose -f docker-compose.staging.yml ps redis | grep -q 'Up'"

# Summary
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Test Summary:"
echo "   Total:  $test_count"
echo -e "   ${GREEN}Passed: $pass_count${NC}"
if [ $fail_count -gt 0 ]; then
    echo -e "   ${RED}Failed: $fail_count${NC}"
else
    echo "   Failed: $fail_count"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [ $fail_count -eq 0 ]; then
    echo -e "${GREEN}✅ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed${NC}"
    echo ""
    echo "💡 Troubleshooting:"
    echo "   - Check logs: docker compose -f docker-compose.staging.yml logs"
    echo "   - Check service status: docker compose -f docker-compose.staging.yml ps"
    echo "   - Restart services: docker compose -f docker-compose.staging.yml restart"
    exit 1
fi
