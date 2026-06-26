#!/bin/bash

# MSF Bennett Command Controller
# Usage: msf bennett <action> bennett-studio <environment>

USER_NAME="bennett"
ACTION="$2"
PROJECT="$3"
ENV="$4"

# ============================================================================
# Help message function
# ============================================================================
show_help() {
    echo ""
    echo "╔══════════════════════════════════════════════════════════════════════╗"
    echo "║           MSF Bennett Studio Development Commands                    ║"
    echo "╚══════════════════════════════════════════════════════════════════════╝"
    echo ""
    echo "Usage: msf bennett <action> <project> <env> [options]"
    echo "       bennett <action> <project> <env> [options]"
    echo ""
    echo "Projects:"
    echo "  bennett-studio    Bennett Studio (desktop + web + engine)"
    echo "  oshocks           Oshocks project"
    echo ""
    echo "Environments:"
    echo "  dev               Development mode"
    echo "  prod              Production mode (not yet implemented)"
    echo ""
    echo "Actions:"
    echo ""
    echo "  ┌────────────────────────────────────────────────────────────────┐"
    echo "  │ START / STOP                                                   │"
    echo "  ├────────────────────────────────────────────────────────────────┤"
    echo "  │ start                    Fast start (~2-130s)                  │"
    echo "  │                          Builds engine only if binary missing  │"
    echo "  │                                                                │"
    echo "  │ restart                  Stop + start (~2-130s)                │"
    echo "  │                          Preserves binary, prompts for Docker  │"
    echo "  │                                                                │"
    echo "  │ restart --with-docker    Restart + restart Docker containers   │"
    echo "  │                                                                │"
    echo "  │ rebuild                  Full clean rebuild (~6-10min)         │"
    echo "  │                          cargo clean + build + start           │"
    echo "  │                                                                │"
    echo "  │ build                    Compile engine only (~2-5min)         │"
    echo "  │                          Does not start servers                │"
    echo "  │                                                                │"
    echo "  │ stop                     Stop all servers + Docker (optional)  │"
    echo "  └────────────────────────────────────────────────────────────────┘"
    echo ""
    echo "  ┌────────────────────────────────────────────────────────────────┐"
    echo "  │ CLEAR / CLEAN                                                  │"
    echo "  ├────────────────────────────────────────────────────────────────┤"
    echo "  │ clear                    Clear logs only (preserves binary)    │"
    echo "  │                          Next start is fast                    │"
    echo "  │                                                                │"
    echo "  │ clear-all                Clear logs + cargo clean              │"
    echo "  │                          DESTRUCTIVE — forces full rebuild     │"
    echo "  └────────────────────────────────────────────────────────────────┘"
    echo ""
    echo "  ┌────────────────────────────────────────────────────────────────┐"
    echo "  │ MONITOR / DEBUG                                                │"
    echo "  ├────────────────────────────────────────────────────────────────┤"
    echo "  │ status                   Show running status of all services   │"
    echo "  │                                                                │"
    echo "  │ logs [service]           Tail logs (default: all)              │"
    echo "  │                          Services: engine, web, desktop, docker│"
    echo "  │                                                                │"
    echo "  │ attach                   Attach to tmux session                │"
    echo "  │                          Ctrl+B then D to detach               │"
    echo "  │                                                                │"
    echo "  │ tree                     Show project file tree                │"
    echo "  └────────────────────────────────────────────────────────────────┘"
    echo ""
    echo "Examples:"
    echo "  msf bennett start bennett-studio dev"
    echo "  msf bennett restart bennett-studio dev"
    echo "  msf bennett restart --with-docker bennett-studio dev"
    echo "  msf bennett stop bennett-studio dev"
    echo "  msf bennett clear bennett-studio dev"
    echo "  msf bennett status bennett-studio dev"
    echo "  msf bennett logs bennett-studio dev engine"
    echo "  msf bennett attach bennett-studio dev"
    echo ""
    echo "  bennett start bennett-studio dev"
    echo "  bennett restart bennett-studio dev"
    echo "  bennett stop bennett-studio dev"
    echo ""
    echo "Tips:"
    echo "  • First start after clear-all or rebuild: ~6-10 minutes"
    echo "  • Daily start with existing binary: ~2-130 seconds"
    echo "  • Engine startup time depends on Docker container scanning"
    echo "  • Use Ctrl+C to exit logs, Ctrl+B then D to detach from tmux"
    echo ""
}

if [ "$1" != "$USER_NAME" ]; then
    show_help
    exit 1
fi

# Handle --help explicitly (for when called directly, not via ~/.bashrc)
if [ "$ACTION" = "--help" ] || [ "$ACTION" = "-h" ]; then
    show_help
    exit 0
fi

if [ -z "$ACTION" ] || [ -z "$PROJECT" ] || [ -z "$ENV" ]; then
    show_help
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
# CRITICAL: Kill all engine processes to prevent SQLite lock conflicts
# ============================================================================
kill_all_engines() {
    echo "🔧 Ensuring no engine processes are running..."

    # Kill by PID file
    if [ -f /tmp/bennett-engine.pid ]; then
        local pid=$(cat /tmp/bennett-engine.pid)
        if kill -0 "$pid" 2>/dev/null; then
            kill -9 "$pid" 2>/dev/null
            sleep 1
        fi
        rm -f /tmp/bennett-engine.pid
    fi

    # Kill ALL bennett-engine processes (catches orphans)
    local pids=$(pgrep -f "bennett-engine" 2>/dev/null || true)
    if [ -n "$pids" ]; then
        echo "   Killing orphaned processes: $pids"
        pkill -9 -f "bennett-engine" 2>/dev/null || true
        sleep 2
    fi

    # Verify they're dead
    if pgrep -f "bennett-engine" > /dev/null 2>&1; then
        echo "⚠️  Some engine processes still running!"
        pkill -9 -f "bennett-engine" 2>/dev/null || true
        sleep 1
    fi

    echo "✅ All engine processes stopped"
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
          # RESTART: Fast restart — stop + start (NO rebuild unless binary missing)
          # ------------------------------------------------------------------
          restart)
            echo "=========================================="
            echo "  🔄 Restarting Bennett Studio Dev Servers"
            echo "=========================================="
            echo ""

            # Check for --with-docker flag (non-interactive mode)
            RESTART_DOCKER_CONTAINERS=false
            for arg in "$@"; do
                if [ "$arg" = "--with-docker" ]; then
                    RESTART_DOCKER_CONTAINERS=true
                fi
            done

            # Kill tmux session
            if tmux has-session -t bennett-studio-dev 2>/dev/null; then
              echo "📦 Stopping tmux session..."
              tmux kill-session -t bennett-studio-dev
            fi

            # CRITICAL: Kill ALL engine processes to release SQLite lock
            kill_all_engines

            # Ask if user wants to restart Docker containers too
            echo ""
            while true; do
                read -rp "🐳 Restart Docker database containers too? [y/N]: " restart_docker
                case "$restart_docker" in
                    [Yy]*)
                        echo "🐳 Restarting Bennett containers..."
                        docker restart $(docker ps -q --filter "label=bennett-managed=true") 2>/dev/null || echo "   No containers to restart"
                        echo "✅ Containers restarted"
                        break
                        ;;
                    [Nn]*|"")
                        echo "🐳 Leaving containers running"
                        break
                        ;;
                    *)
                        echo "Please answer y or n"
                        ;;
                esac
            done
            echo ""

            echo "🌐 Stopping Web..."
            "$PROJECT_DIR/scripts/web-dev-control" stop 2>/dev/null || true
            echo "🖥️  Stopping Desktop..."
            "$PROJECT_DIR/scripts/desktop-dev-control" stop 2>/dev/null || true
            echo ""

            # Only rebuild if binary is missing (e.g., after cargo clean)
            if [ ! -x "$BINARY" ]; then
                echo "⚠️  Engine binary missing. Rebuilding..."
                if ! build_engine; then
                    exit 1
                fi
                echo ""
            else
                echo "✅ Using existing binary: $BINARY"
                echo "   (Run 'msf bennett rebuild bennett-studio dev' for full clean rebuild)"
                echo ""
            fi

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

            # Kill tmux session
            if tmux has-session -t bennett-studio-dev 2>/dev/null; then
              echo "📦 Stopping tmux session..."
              tmux kill-session -t bennett-studio-dev
            fi

            # CRITICAL: Kill ALL engine processes
            kill_all_engines

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

            # CRITICAL: Kill ALL engine processes
            kill_all_engines

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
          # CLEAR: Clear logs only (does NOT destroy binary)
          # ------------------------------------------------------------------
          clear)
            echo "=========================================="
            echo "  🧹 Clearing Bennett Studio Dev Logs"
            echo "=========================================="
            echo ""

            clear_docker
            echo ""

            echo "🧹 Clearing Engine logs..."
            "$PROJECT_DIR/scripts/engine-control" clear 2>/dev/null || echo "Engine clear not available"
            echo ""

            echo "🧹 Clearing Web logs..."
            "$PROJECT_DIR/scripts/web-dev-control" clear
            echo ""

            echo "🧹 Clearing Desktop logs..."
            "$PROJECT_DIR/scripts/desktop-dev-control" clear
            echo ""

            echo "=========================================="
            echo "  ✅ All logs cleared!"
            echo "   Binary preserved: $BINARY"
            echo "   Next start will be fast (~2 seconds)"
            echo "=========================================="
            echo ""
            echo "   To also clear build cache (forces full rebuild):"
            echo "   msf bennett clear-all bennett-studio dev"
            echo "=========================================="
            ;;

          # ------------------------------------------------------------------
          # CLEAR-ALL: Clear logs + cargo clean (DESTRUCTIVE — forces rebuild)
          # ------------------------------------------------------------------
          clear-all)
            echo "=========================================="
            echo "  🧹🧹 CLEARING ALL — Logs + Build Cache"
            echo "=========================================="
            echo ""
            echo "⚠️  WARNING: This will delete the compiled binary!"
            echo "   Next start will require a full rebuild (~6-10 minutes)."
            echo ""

            # Run regular clear first
            clear_docker
            echo ""

            echo "🧹 Clearing Engine logs..."
            "$PROJECT_DIR/scripts/engine-control" clear 2>/dev/null || true
            echo ""

            echo "🧹 Clearing Web logs..."
            "$PROJECT_DIR/scripts/web-dev-control" clear
            echo ""

            echo "🧹 Clearing Desktop logs..."
            "$PROJECT_DIR/scripts/desktop-dev-control" clear
            echo ""

            echo "🧹 Clearing cargo build cache..."
            cd "$PROJECT_DIR" || exit 1
            cargo clean 2>/dev/null || echo "Cargo clean not available"
            echo ""

            echo "=========================================="
            echo "  ✅ All logs AND build cache cleared!"
            echo "   Binary deleted: $BINARY"
            echo "   Next start will require full rebuild (~6-10 minutes)"
            echo "=========================================="
            echo ""
            echo "   To rebuild and start:"
            echo "   msf bennett rebuild bennett-studio dev"
            echo "=========================================="
            ;;

          # ------------------------------------------------------------------
          # STATUS
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

          # ------------------------------------------------------------------
          # LOGS: Tail logs with optional service filter
          # Usage: msf bennett logs bennett-studio dev [engine|web|desktop|docker|all]
          # ------------------------------------------------------------------
          logs)
            LOG_SERVICE="${5:-all}"

            echo "=========================================="
            echo "  📜 Tailing Bennett Studio Dev Logs"
            echo "=========================================="
            echo ""

            case "$LOG_SERVICE" in
              engine|e)
                echo "🔧 Tailing Engine log only..."
                echo "   Press Ctrl+C to stop"
                echo ""
                tail -f /tmp/bennett-engine.log 2>/dev/null || echo "❌ Engine log not found"
                ;;
              web|w)
                echo "🌐 Tailing Web log only..."
                echo "   Press Ctrl+C to stop"
                echo ""
                tail -f /tmp/bennett-web.log 2>/dev/null || echo "❌ Web log not found"
                ;;
              desktop|d)
                echo "🖥️  Tailing Desktop log only..."
                echo "   Press Ctrl+C to stop"
                echo ""
                tail -f /tmp/bennett-desktop.log 2>/dev/null || echo "❌ Desktop log not found"
                ;;
              docker|dockerd|doc)
                echo "🐳 Tailing Docker log only..."
                echo "   Press Ctrl+C to stop"
                echo ""
                tail -f /tmp/dockerd.log 2>/dev/null || echo "❌ Docker log not found"
                ;;
              all|a|*)
                echo "📜 Tailing ALL logs..."
                echo "   Press Ctrl+C to stop"
                echo ""
                tail -f /tmp/bennett-engine.log /tmp/bennett-web.log /tmp/bennett-desktop.log /tmp/dockerd.log 2>/dev/null || echo "Some log files not found. Servers may not be running."
                ;;
            esac
            ;;

          # ------------------------------------------------------------------
          # ATTACH
          # ------------------------------------------------------------------
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

          # ------------------------------------------------------------------
          # TREE
          # ------------------------------------------------------------------
          tree)
            echo "=========================================="
            echo "  📁 Bennett Studio Project Tree"
            echo "=========================================="
            echo ""
            cd "$PROJECT_DIR" || exit 1
            tree -L 5 -I 'node_modules|dist|build|target|vendor|.git|storage|*.sqlite'
            ;;

          --help|-h)
            show_help
            exit 0
            ;;

          *)
            echo ""
            echo "❌ Unknown action: '$ACTION'"
            echo ""
            show_help
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
