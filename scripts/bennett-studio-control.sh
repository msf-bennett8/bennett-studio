#!/bin/bash

# MSF Bennett Command Controller
# Usage: msf bennett <action> bennett-studio <environment>

USER_NAME="bennett"
ACTION="$2"
PROJECT="$3"
ENV="$4"

if [ "$1" != "$USER_NAME" ]; then
    echo "Usage: msf bennett <start|stop|clear|restart|status|logs|attach|tree> <project> <dev|prod>"
    exit 1
fi

if [ -z "$ACTION" ] || [ -z "$PROJECT" ] || [ -z "$ENV" ]; then
    echo "Usage: msf bennett <start|stop|clear|restart|status|logs|attach|tree> <project> <dev|prod>"
    exit 1
fi

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

    # Give user 15 seconds to start docker in another terminal
    echo "⏳ Waiting 15 seconds for Docker to start (Ctrl+C to cancel)..."
    for i in 27 26 25 24 23 22 21 20 19 18 17 16 15 14 13 12 11 10 9 8 7 6 5 4 3 2 1; do
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

start_docker() {
    if docker_is_running; then
        if ask_docker "Docker is already running. Restart?"; then
            echo "🐳 Restarting Docker daemon..."
            if tmux has-session -t docker 2>/dev/null; then
                tmux kill-session -t docker 2>/dev/null
                sleep 1
            fi
            sudo pkill -x dockerd 2>/dev/null
            sleep 2
            echo "🐳 Please start Docker manually: sudo dockerd"
            echo "   (Cannot auto-start dockerd without passwordless sudo)"
            return 1
        else
            echo "🐳 Docker left running."
        fi
    else
        echo "🐳 Docker is not running."
        echo "   Please start it first: sudo dockerd"
        return 1
    fi
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

case "$PROJECT" in
  oshocks)
    # Delegate to existing oshocks controller
    ~/studio.dev/oshocks/scripts/oshocks-control.sh "$@"
    ;;
  bennett-studio)
    case "$ENV" in
      dev)
        case "$ACTION" in
          start)
            echo "=========================================="
            echo "  🚀 Starting Bennett Studio Dev Servers"
            echo "=========================================="
            echo ""

            # Docker check
            if ! check_docker_required; then
                exit 1
            fi
            echo ""

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
            ~/studio.dev/bennett\ studio/scripts/engine-control stop 2>/dev/null || true
            echo ""

            echo "🌐 Stopping Web..."
            ~/studio.dev/bennett\ studio/scripts/web-dev-control stop 2>/dev/null || true
            echo ""

            echo "🖥️  Stopping Desktop..."
            ~/studio.dev/bennett\ studio/scripts/desktop-dev-control stop 2>/dev/null || true
            echo ""

            # Docker prompt
            stop_docker
            echo ""

            echo "=========================================="
            echo "  ✅ All servers stopped!"
            echo "=========================================="
            ;;
          clear)
            echo "=========================================="
            echo "  🧹 Clearing Bennett Studio Dev Caches"
            echo "=========================================="
            echo ""

            # Docker prompt
            clear_docker
            echo ""

            echo "🧹 Clearing Engine..."
            ~/studio.dev/bennett\ studio/scripts/engine-control clear 2>/dev/null || echo "Engine clear not available"
            echo ""

            echo "🧹 Clearing Web..."
            ~/studio.dev/bennett\ studio/scripts/web-dev-control clear
            echo ""

            echo "🧹 Clearing Desktop..."
            ~/studio.dev/bennett\ studio/scripts/desktop-dev-control clear
            echo ""

            echo "🧹 Clearing cargo build cache..."
            cd ~/studio.dev/bennett\ studio || exit 1
            cargo clean 2>/dev/null || echo "Cargo clean not available"
            echo ""

            echo "=========================================="
            echo "  ✅ All caches cleared!"
            echo "=========================================="
            ;;
          restart)
            "$0" bennett stop bennett-studio dev
            sleep 1
            "$0" bennett clear bennett-studio dev
            sleep 1
            "$0" bennett start bennett-studio dev
            ;;
          status)
            echo "=========================================="
            echo "  📊 Bennett Studio Dev Status"
            echo "=========================================="
            echo ""
            docker_status
            ~/studio.dev/bennett\ studio/scripts/engine-control status
            ~/studio.dev/bennett\ studio/scripts/web-dev-control status
            ~/studio.dev/bennett\ studio/scripts/desktop-dev-control status
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
            cd ~/studio.dev/bennett\ studio || exit 1
            tree -L 5 -I 'node_modules|dist|build|target|vendor|.git|storage|*.sqlite'
            ;;
          *)
            echo "Usage: msf bennett <start|stop|clear|restart|status|logs|attach|tree> bennett-studio dev"
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
