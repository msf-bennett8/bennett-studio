#!/bin/bash

exec < /dev/null
PROJECT_DIR="/home/msf_bennett/studio.dev/bennett studio"

echo "=========================================="
echo "  🚀 Starting Bennett Studio Dev Servers"
echo "=========================================="
echo ""

echo "🧹 Cleaning up zombie processes..."
fuser -k 5173/tcp 2>/dev/null || true
fuser -k 5174/tcp 2>/dev/null || true
pkill -f "vite.*port 517[34]" 2>/dev/null || true
pkill -f "tauri.*dev" 2>/dev/null || true
sleep 2

echo "🔧 Starting Engine..."
"$PROJECT_DIR/scripts/engine-control" start

# Read the ACTUAL port the engine bound to
ENGINE_PORT=$(cat /tmp/bennett-engine.port 2>/dev/null || echo "3001")

echo ""
echo "⏳ Waiting for health check on port $ENGINE_PORT..."
for i in {1..60}; do
    if curl -s "http://localhost:$ENGINE_PORT/api/health" > /dev/null 2>&1; then
        echo "✅ Engine ready on http://localhost:$ENGINE_PORT"
        break
    fi
    sleep 1
    if [ $i -eq 60 ]; then
        echo "❌ Engine health check failed. Check /tmp/bennett-engine.log"
        exit 1
    fi
done
echo ""

echo "🌐 Starting Web..."
"$PROJECT_DIR/scripts/web-dev-control" start
echo ""

echo "🖥️  Starting Desktop..."
cd "$PROJECT_DIR/desktop" || exit 1
npm run tauri dev

echo ""
echo "🛑 Desktop stopped. Cleaning up..."
"$PROJECT_DIR/scripts/web-dev-control" stop 2>/dev/null || true
"$PROJECT_DIR/scripts/engine-control" stop 2>/dev/null || true
echo "✅ All services stopped."
