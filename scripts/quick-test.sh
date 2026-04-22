#!/bin/bash
# Quick test script to verify PolyPulse is working

set -e

echo "PolyPulse Quick Test"
echo "======================"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
PASSED=0
FAILED=0

test_service() {
    local name=$1
    local url=$2
    
    echo -n "Testing $name... "
    if curl -sf "$url" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ PASS${NC}"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}✗ FAIL${NC}"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

test_docker_service() {
    local name=$1
    local service=$2
    
    echo -n "Testing $name... "
    if docker compose ps | grep -q "$service.*Up"; then
        echo -e "${GREEN}✓ PASS${NC}"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}✗ FAIL${NC}"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

echo "Docker Services"
echo "------------------"
test_docker_service "PostgreSQL" "db"
test_docker_service "Redis" "redis"
test_docker_service "Backend" "backend"
test_docker_service "Frontend" "frontend"
echo ""

echo "HTTP Endpoints"
echo "-----------------"
test_service "Frontend" "http://localhost:5173"
test_service "Backend API" "http://localhost:8000/api/polls/stats/"
test_service "Django Admin" "http://localhost:8000/admin/login/"
echo ""

echo "Database"
echo "------------"
echo -n "Testing migrations... "
if docker compose exec -T backend python manage.py showmigrations | grep -q "\[X\]"; then
    echo -e "${GREEN}✓ PASS${NC}"
    PASSED=$((PASSED + 1))
else
    echo -e "${RED}✗ FAIL${NC}"
    FAILED=$((FAILED + 1))
fi
echo ""

echo "Authentication"
echo "-----------------"
echo -n "Testing superuser exists... "
if docker compose exec -T backend python manage.py shell -c "
from django.contrib.auth.models import User
exit(0 if User.objects.filter(is_superuser=True).exists() else 1)
" 2>/dev/null; then
    echo -e "${GREEN}✓ PASS${NC}"
    PASSED=$((PASSED + 1))
else
    echo -e "${YELLOW}⚠ SKIP (run setup script)${NC}"
fi
echo ""

echo "Results"
echo "----------"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Open http://localhost:5173 in your browser"
    echo "  2. Login with admin/admin123"
    echo "  3. Create a market via Django Admin"
    echo "  4. Start trading!"
    exit 0
else
    echo -e "${RED}Some tests failed${NC}"
    echo ""
    echo "Troubleshooting:"
    echo "  1. Check if services are running: docker compose ps"
    echo "  2. Check logs: docker compose logs"
    echo "  3. Run setup: bash scripts/codespaces-setup.sh"
    exit 1
fi
