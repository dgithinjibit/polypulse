#!/bin/bash
set -e

# PolyPulse Staging Deployment Script
# This script deploys the application to the staging environment

echo "🚀 Starting PolyPulse Staging Deployment..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if .env file exists
if [ ! -f .env ]; then
    echo -e "${RED}❌ Error: .env file not found${NC}"
    echo "Please copy .env.staging to .env and configure it with staging values"
    exit 1
fi

# Validate required environment variables
required_vars=("JWT_SECRET" "POSTGRES_PASSWORD" "VITE_API_URL")
for var in "${required_vars[@]}"; do
    if ! grep -q "^${var}=" .env || grep -q "CHANGE_ME" .env; then
        echo -e "${RED}❌ Error: ${var} not properly configured in .env${NC}"
        exit 1
    fi
done

echo -e "${GREEN}✓ Environment configuration validated${NC}"

# Pull latest code (if in a git repository)
if [ -d .git ]; then
    echo "📥 Pulling latest code..."
    git pull origin main || echo -e "${YELLOW}⚠ Warning: Could not pull latest code${NC}"
fi

# Stop existing containers
echo "🛑 Stopping existing containers..."
docker compose -f docker-compose.staging.yml down

# Build and start services
echo "🔨 Building and starting services..."
docker compose -f docker-compose.staging.yml up -d --build

# Wait for database to be ready
echo "⏳ Waiting for database to be ready..."
timeout 60 bash -c 'until docker compose -f docker-compose.staging.yml exec -T db pg_isready -U polypulse -d polypulse_staging 2>/dev/null; do sleep 2; done' || {
    echo -e "${RED}❌ Database failed to start${NC}"
    docker compose -f docker-compose.staging.yml logs db
    exit 1
}

echo -e "${GREEN}✓ Database is ready${NC}"

# Run database migrations
echo "🔄 Running database migrations..."
docker compose -f docker-compose.staging.yml exec -T backend sh -c "cd /app && sqlx migrate run" || {
    echo -e "${YELLOW}⚠ Warning: Migrations may have failed or already applied${NC}"
}

# Wait for backend to be healthy
echo "⏳ Waiting for backend to be healthy..."
for i in {1..30}; do
    if curl -f http://localhost:8000/health >/dev/null 2>&1; then
        echo -e "${GREEN}✓ Backend is healthy${NC}"
        break
    fi
    if [ $i -eq 30 ]; then
        echo -e "${RED}❌ Backend failed to become healthy${NC}"
        docker compose -f docker-compose.staging.yml logs backend
        exit 1
    fi
    sleep 2
done

# Check frontend
echo "⏳ Checking frontend..."
for i in {1..15}; do
    if curl -f http://localhost:5173 >/dev/null 2>&1; then
        echo -e "${GREEN}✓ Frontend is responding${NC}"
        break
    fi
    if [ $i -eq 15 ]; then
        echo -e "${YELLOW}⚠ Warning: Frontend may not be responding${NC}"
        docker compose -f docker-compose.staging.yml logs frontend
    fi
    sleep 2
done

# Display service status
echo ""
echo "📊 Service Status:"
docker compose -f docker-compose.staging.yml ps

echo ""
echo -e "${GREEN}✅ Staging deployment complete!${NC}"
echo ""
echo "🌐 Services:"
echo "   Backend:  http://localhost:8000"
echo "   Frontend: http://localhost:5173"
echo ""
echo "📝 Next steps:"
echo "   1. Test the deployment: ./scripts/test-staging.sh"
echo "   2. View logs: docker compose -f docker-compose.staging.yml logs -f"
echo "   3. Monitor health: curl http://localhost:8000/health"
echo ""
