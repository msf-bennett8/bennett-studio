#!/bin/bash

# MSF Bennett Command Controller
# Usage: msf bennett <action> bennett-studio <environment>

USER_NAME="bennett"
ACTION="$2"
PROJECT="$3"
ENV="$4"

if [ "$1" != "$USER_NAME" ]; then
    echo "Usage: msf bennett <start|restart|rebuild|stop|clear|build|status|logs|attach|tree> <project> <dev|prod>"
    exit 1
fi

if [ -z "$ACTION" ] || [ -z "$PROJECT" ] || [ -z "$ENV" ]; then
    echo "Usage: msf bennett <start|restart|rebuild|stop|clear|build|status|logs|attach|tree> <project> <dev|prod>"
    exit 1
fi

PROJECT_DIR="/home/msf_bennett/studio.dev/bennett studio"
ENGINE_DIR="$PROJECT_DIR/engine"
BINARY="$PROJECT_DIR/target/debug/bennett-engine"

# Docker helper functions
ask_docker() {
    local prompt="$1"
    while true; do
        read -rp "$prompt [Y/n/Yes/No]: " answer
        case "$answer" in
            [Yy]|[Yy][Ee][Ss]) return 0 ;;
            [Nn]|[Nn][Oo]) return 1 ;;
            *) echo "Please enter Y, Yes, N, or No." ;;
        esac
    done
}

docker_is_running() {
    pgrep -x "dockerd" > /dev/null 2>&1
}

docker_status() {
    if docker_is_running; then
        echo "🐳 Docker: RUNNING"
    else
        echo "🐳 Docker: STOPPED"
    fi
}

check_docker_required() {
    if docker_is_running; then
        return 0
    fi

    echo ""
    echo "❌ Docker is NOT running."
    echo ""
    echo "   The engine needs Docker to manage database containers."
    echo ""
    echo "   Quick start:"
    echo "   sudo dockerd"
    echo ""
    echo "   Or in background:"
    echo "   tmux new-session -s docker 'sudo dockerd'"
    echo ""

    echo "⏳ Waiting 15 seconds for Docker to start (Ctrl+C to cancel)..."
    for i in 15 14 13 12 11 10 9 8 7 6 5 4 3 2 1; do
        if docker_is_running; then
            echo ""
            echo "✅ Docker detected! Proceeding..."
            return 0
        fi
        echo -ne "\r  $i seconds remaining... "
        sleep 1
    done
    echo ""
    echo "❌ Docker still not running. Start it manually, then retry."
    echo ""
    return 1
}

stop_docker() {
    if docker_is_running; then
        if ask_docker "Stop Docker?"; then
            echo "🐳 Stopping Docker daemon..."
            if tmux has-session -t docker 2>/dev/null; then
                tmux kill-session -t docker 2>/dev/null
            fi
            sudo pkill -x dockerd 2>/dev/null
            sleep 1
            echo "✅ Docker daemon stopped"
        else
            echo "🐳 Docker left running."
        fi
    else
        echo "🐳 Docker is not running."
    fi
}

clear_docker() {
    if ask_docker "Clear Docker logs?"; then
        rm -f /tmp/dockerd.log 2>/dev/null
        echo "🐳 Docker logs cleared"
    else
        echo "🐳 Docker logs left intact."
    fi
}

# ============================================================================
# Engine Build Helpers
# ============================================================================

build_engine() {
    echo "🔨 Building Bennett Engine..."
    echo "   Binary: $BINARY"
    echo ""

    # Kill any stale cargo processes that might hold the build lock
    pkill -f "cargo.*bennett" 2>/dev/null || true
    pkill -f "rustc.*bennett" 2>/dev/null || true
    pkill -f "rust-lld" 2>/dev/null || true
    sleep 2

    cd "$ENGINE_DIR" || exit 1
    if cargo build --bin bennett-engine 2>&1 | tee /tmp/bennett-engine-build.log; then
        echo ""
        echo "✅ Engine built successfully"
        return 0
    else
        echo ""
        echo "❌ Engine build failed!"
        echo "   Check: cat /tmp/bennett-engine-build.log"
        return 1
    fi
}

# ============================================================================
# Main Actions
# ============================================================================

case "$PROJECT" in
  oshocks)
    ~/studio.dev/oshocks/scripts/oshocks-control.sh "$@"
    ;;
  bennett-studio)
    case "$ENV" in
      dev)
        case "$ACTION" in
          # ------------------------------------------------------------------
          # START: Fast start — build only if binary missing
          # ------------------------------------------------------------------
          start)
            echo "=========================================="
            echo "  🚀 Starting Bennett Studio Dev Servers"
            echo "=========================================="
            echo ""

            if ! check_docker_required; then
                exit 1
            fi
            echo ""

            # Check if binary exists — build if missing
            if [ ! -x "$BINARY" ]; then
                echo "⚠️  Engine binary not found."
                echo "   Running initial build (this takes ~10 min)..."
                echo ""
                if ! build_engine; then
                    exit 1
                fi
                echo ""
            else
                echo "✅ Using pre-built engine: $BINARY"
                echo ""
            fi

            if tmux has-session -t bennett-studio-dev 2>/dev/null; then
              echo "⚠️  tmux session 'bennett-studio-dev' already exists!"
              echo "   Reattach: tmux attach -t bennett-studio-dev"
              echo "   Kill:     tmux kill-session -t bennett-studio-dev"
              exit 1
            fi

            echo "📦 Creating tmux session 'bennett-studio-dev'..."
            tmux new-session -d -s bennett-studio-dev -n servers
            tmux send-keys -t bennett-studio-dev:0 'bash "/home/msf_bennett/studio.dev/bennett studio/scripts/tmux-bennett-start.sh"' C-m

            echo ""
            echo "=========================================="
            echo "  ✅ All servers started in tmux!"
            echo "=========================================="
            echo ""
            echo "  📎 Attach: tmux attach -t bennett-studio-dev"
            echo "  📎 Detach: Ctrl+B, then D"
            echo "  📎 Kill:   tmux kill-session -t bennett-studio-dev"
            echo ""
            echo "  Services:"
            echo "    🐳 Docker:   $(docker_status | cut -d' ' -f2-)"
            echo "    🔧 Engine:   http://localhost:3000"
            echo "    🌐 Web:      http://localhost:5173"
            echo "    🖥️  Desktop:  http://localhost:5174"
            echo ""
            echo "  Servers survive terminal closes!"
            echo "=========================================="
            ;;

          # ------------------------------------------------------------------
          # RESTART: Stop + rebuild engine + start (keeps caches)
          # ------------------------------------------------------------------
          restart)
            echo "=========================================="
            echo "  🔄 Restarting Bennett Studio Dev Servers"
            echo "=========================================="
            echo ""

            # Stop everything
            if tmux has-session -t bennett-studio-dev 2>/dev/null; then
              echo "📦 Stopping tmux session..."
              tmux kill-session -t bennett-studio-dev
            fi
            echo "🔧 Stopping Engine..."
            "$PROJECT_DIR/scripts/engine-control" stop 2>/dev/null || true
            echo "🌐 Stopping Web..."
            "$PROJECT_DIR/scripts/web-dev-control" stop 2>/dev/null || true
            echo "🖥️  Stopping Desktop..."
            "$PROJECT_DIR/scripts/desktop-dev-control" stop 2>/dev/null || true
            echo ""

            # Rebuild engine (does NOT run cargo clean — keeps dependency cache)
            echo "🔨 Rebuilding engine..."
            if ! build_engine; then
                exit 1
            fi
            echo ""

            # Start fresh
            echo "🚀 Starting servers..."
            "$0" bennett start bennett-studio dev
            ;;

          # ------------------------------------------------------------------
          # REBUILD: Full clean rebuild — use when dependencies change
          # ------------------------------------------------------------------
          rebuild)
            echo "=========================================="
            echo "  🧱 Full Rebuild Bennett Studio Dev"
            echo "=========================================="
            echo ""

            # Stop everything
            if tmux has-session -t bennett-studio-dev 2>/dev/null; then
              echo "📦 Stopping tmux session..."
              tmux kill-session -t bennett-studio-dev
            fi
            "$PROJECT_DIR/scripts/engine-control" stop 2>/dev/null || true
            "$PROJECT_DIR/scripts/web-dev-control" stop 2>/dev/null || true
            "$PROJECT_DIR/scripts/desktop-dev-control" stop 2>/dev/null || true
            echo ""

            # Clean and rebuild
            echo "🧹 Running cargo clean..."
            cd "$PROJECT_DIR" || exit 1
            cargo clean
            echo ""

            echo "🔨 Building engine from scratch..."
            if ! build_engine; then
                exit 1
            fi
            echo ""

            # Start fresh
            echo "🚀 Starting servers..."
            "$0" bennett start bennett-studio dev
            ;;

          # ------------------------------------------------------------------
          # BUILD: Compile engine only, don't start servers
          # ------------------------------------------------------------------
          build)
            echo "=========================================="
            echo "  🔨 Building Bennett Studio Engine"
            echo "=========================================="
            echo ""

            if ! check_docker_required; then
                exit 1
            fi
            echo ""

            if ! build_engine; then
                exit 1
            fi
            echo ""
            echo "✅ Build complete. Start servers with:"
            echo "   msf bennett start bennett-studio dev"
            echo "=========================================="
            ;;

          # ------------------------------------------------------------------
          # STOP
          # ------------------------------------------------------------------
          stop)
            echo "=========================================="
            echo "  🛑 Stopping Bennett Studio Dev Servers"
            echo "=========================================="
            echo ""

            if tmux has-session -t bennett-studio-dev 2>/dev/null; then
              echo "📦 Killing tmux session..."
              tmux kill-session -t bennett-studio-dev
            fi

            echo "🔧 Stopping Engine..."
            "$PROJECT_DIR/scripts/engine-control" stop 2>/dev/null || true
            echo ""

            echo "🌐 Stopping Web..."
            "$PROJECT_DIR/scripts/web-dev-control" stop 2>/dev/null || true
            echo ""

            echo "🖥️  Stopping Desktop..."
            "$PROJECT_DIR/scripts/desktop-dev-control" stop 2>/dev/null || true
            echo ""

            stop_docker
            echo ""

            echo "=========================================="
            echo "  ✅ All servers stopped!"
            echo "=========================================="
            ;;

          # ------------------------------------------------------------------
          # CLEAR: Clear logs and caches (does NOT cargo clean anymore)
          # ------------------------------------------------------------------
          clear)
            echo "=========================================="
            echo "  🧹 Clearing Bennett Studio Dev Caches"
            echo "=========================================="
            echo ""

            clear_docker
            echo ""

            echo "🧹 Clearing Engine..."
            "$PROJECT_DIR/scripts/engine-control" clear 2>/dev/null || echo "Engine clear not available"
            echo ""

            echo "🧹 Clearing Web..."
            "$PROJECT_DIR/scripts/web-dev-control" clear
            echo ""

            echo "🧹 Clearing Desktop..."
            "$PROJECT_DIR/scripts/desktop-dev-control" clear
            echo ""

            echo "🧹 Clearing cargo build cache..."
            cd "$PROJECT_DIR" || exit 1
            cargo clean 2>/dev/null || echo "Cargo clean not available"
            echo ""

            echo "=========================================="
            echo "  ✅ All caches cleared!"
            echo "   Note: Next start will require full rebuild (~10 min)"
            echo "=========================================="
            ;;

          # ------------------------------------------------------------------
          # STATUS, LOGS, ATTACH, TREE
          # ------------------------------------------------------------------
          status)
            echo "=========================================="
            echo "  📊 Bennett Studio Dev Status"
            echo "=========================================="
            echo ""
            docker_status
            "$PROJECT_DIR/scripts/engine-control" status
            "$PROJECT_DIR/scripts/web-dev-control" status
            "$PROJECT_DIR/scripts/desktop-dev-control" status
            echo ""
            if [ -x "$BINARY" ]; then
                echo "✅ Engine binary: $BINARY"
            else
                echo "⚠️  Engine binary not found — run: msf bennett build bennett-studio dev"
            fi
            echo ""
            echo "=========================================="
            ;;

          logs)
            echo "=========================================="
            echo "  📜 Tailing Bennett Studio Dev Logs"
            echo "=========================================="
            echo ""
            echo "Press Ctrl+C to stop viewing logs"
            echo ""
            tail -f /tmp/bennett-engine.log /tmp/bennett-web.log /tmp/bennett-desktop.log /tmp/dockerd.log 2>/dev/null || echo "Some log files not found. Servers may not be running."
            ;;

          attach)
            if tmux has-session -t bennett-studio-dev 2>/dev/null; then
              echo "📎 Attaching to bennett-studio-dev tmux session..."
              tmux attach -t bennett-studio-dev
            else
              echo "❌ No tmux session 'bennett-studio-dev' found."
              echo "   Start servers first: msf bennett start bennett-studio dev"
              exit 1
            fi
            ;;

          tree)
            echo "=========================================="
            echo "  📁 Bennett Studio Project Tree"
            echo "=========================================="
            echo ""
            cd "$PROJECT_DIR" || exit 1
            tree -L 5 -I 'node_modules|dist|build|target|vendor|.git|storage|*.sqlite'
            ;;

          *)
            echo "Usage: msf bennett <start|restart|rebuild|stop|clear|build|status|logs|attach|tree> bennett-studio dev"
            exit 1
            ;;
        esac
        ;;
      *)
        echo "Environment '$ENV' not found for bennett-studio. Use 'dev'."
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Project '$PROJECT' not found. Available: oshocks, bennett-studio"
    exit 1
    ;;
esac
