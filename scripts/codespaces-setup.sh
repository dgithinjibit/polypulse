#!/bin/bash
set -e

echo "🚀 PolyPulse Codespaces Setup Script (Rust + Stellar)"
echo "===================================="
echo ""

# Check if docker compose is running
if ! docker compose ps | grep -q "Up"; then
    echo "⚠️  Docker Compose is not running!"
    echo "Please run: docker compose up --build"
    echo "Then run this script again in a new terminal."
    exit 1
fi

echo "✅ Docker Compose is running"
echo ""

# Wait for backend to be ready
echo "⏳ Waiting for Rust backend to be ready..."
max_attempts=30
attempt=0
while ! curl -s http://localhost:8000/health > /dev/null 2>&1; do
    attempt=$((attempt + 1))
    if [ $attempt -ge $max_attempts ]; then
        echo "❌ Backend failed to start after $max_attempts attempts"
        exit 1
    fi
    echo "   Attempt $attempt/$max_attempts..."
    sleep 2
done
echo "✅ Backend is ready"
echo ""

# Run migrations
echo "🔄 Running database migrations..."
docker compose exec -T backend sqlx migrate run || echo "⚠️  Migrations may have already been applied"
echo "✅ Migrations complete"
echo ""

echo "✅ Setup complete!"
echo ""
echo "Access the app:"
echo "   - Frontend: Check the PORTS panel for port 5173"
echo "   - Backend API: Check the PORTS panel for port 8000"
echo "   - Health Check: Backend URL + /health"
echo ""
echo "Next steps:"
echo "   1. Open the PORTS panel (bottom of VS Code)"
echo "   2. Click the globe icon 🌐 next to port 5173"
echo "   3. Connect your Freighter wallet (Stellar)"
echo "   4. Create a poll and start trading!"
echo ""
echo "API Endpoints:"
echo "   - GET  /health"
echo "   - GET  /api/v1/polls"
echo "   - POST /api/v1/auth/login"
echo "   - WS   /ws?token=<jwt>"
echo ""
