#!/bin/bash

# Keep tmux pane alive while monitoring servers
exec < /dev/null

PROJECT_DIR="/home/msf_bennett/studio.dev/bennett studio"

echo "=========================================="
echo "  🚀 Starting Bennett Studio Dev Servers"
echo "=========================================="
echo ""

# Kill zombie Vite/Tauri processes (engine handles its own port conflicts)
echo "🧹 Cleaning up zombie Vite/Tauri processes..."
fuser -k 5173/tcp 2>/dev/null
fuser -k 5174/tcp 2>/dev/null
pkill -f "vite.*port 517[34]" 2>/dev/null
pkill -f "tauri.*dev" 2>/dev/null
sleep 2
echo ""

# Step 1: Start Engine and wait for it to be ready
echo "🔧 Starting Engine..."
"$PROJECT_DIR/scripts/engine-control" start

echo "⏳ Waiting for engine health check..."
for i in {1..120}; do
    if curl -s http://localhost:3001/api/health > /dev/null 2>&1; then
        echo "✅ Engine ready on http://localhost:3001"
        break
    fi
    sleep 1
    if [ $i -eq 120 ]; then
        echo "❌ Engine failed to start. Check /tmp/bennett-engine.log"
        exit 1
    fi
done
echo ""

# Step 2: Start Web in background
echo "🌐 Starting Web..."
"$PROJECT_DIR/scripts/web-dev-control" start
echo ""

# Step 3: Start Desktop in FOREGROUND (Tauri manages Vite)
echo "🖥️  Starting Desktop (Tauri will open window when ready)..."
echo "    This may take 30-60 seconds for first compile..."
echo ""
cd "$PROJECT_DIR/desktop" || exit 1

# Run Tauri in foreground so it can properly manage Vite lifecycle
npm run tauri dev

# When Tauri exits (window closed), stop everything
echo ""
echo "🛑 Desktop stopped. Cleaning up..."
"$PROJECT_DIR/scripts/web-dev-control" stop 2>/dev/null || true
"$PROJECT_DIR/scripts/engine-control" stop 2>/dev/null || true
echo "✅ All services stopped."
