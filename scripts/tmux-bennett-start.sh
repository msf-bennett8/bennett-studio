#!/bin/bash

# Keep tmux pane alive while monitoring servers
exec < /dev/null

# Start services in order: Engine first (web/desktop proxy to it)
~/studio.dev/bennett\ studio/scripts/engine-control start
sleep 3

~/studio.dev/bennett\ studio/scripts/web-dev-control start
sleep 2

~/studio.dev/bennett\ studio/scripts/desktop-dev-control start
sleep 2

echo ""
echo "=========================================="
echo "  ✅ All Bennett Studio servers started!"
echo "=========================================="
echo ""
echo "  🔧 Engine:   http://localhost:${BENNETT_ENGINE_PORT:-3001}"
echo "  🌐 Web:      http://localhost:5173"
echo "  🖥️  Desktop:  http://localhost:5174 (Tauri window)"
echo ""
echo "  Press Ctrl+C to stop all and exit tmux"
echo "=========================================="

# Monitor loop - keep pane alive
while true; do
    if ! [ -f /tmp/bennett-engine.pid ] && ! [ -f /tmp/bennett-web.pid ] && ! [ -f /tmp/bennett-desktop.pid ]; then
        echo "All servers stopped. Exiting..."
        break
    fi
    sleep 5
done
