#!/bin/bash

# MSF Bennett Command Controller
# Usage: msf bennett <action> bennett-studio <environment>

USER_NAME="bennett"
ACTION="$2"
PROJECT="$3"
ENV="$4"

# Color definitions
C_RESET='\033[0m'
C_RED='\033[0;31m'
C_GREEN='\033[0;32m'
C_YELLOW='\033[0;33m'
C_BLUE='\033[0;34m'
C_MAGENTA='\033[0;35m'
C_CYAN='\033[0;36m'
C_WHITE='\033[1;37m'
C_BOLD='\033[1m'
C_DIM='\033[2m'

# ============================================================================
# Help message function
# ============================================================================
show_help() {
    echo ""
    echo -e "${C_BOLD}+----------------------------------------------------------------------+${C_RESET}"
    echo -e "${C_BOLD}|  ${C_CYAN}MSF Bennett Studio Development Commands${C_RESET}${C_BOLD}                            |${C_RESET}"
    echo -e "${C_BOLD}+----------------------------------------------------------------------+${C_RESET}"
    echo ""
    echo -e "${C_DIM}Usage:${C_RESET} ${C_WHITE}msf bennett <action> <project> <env> [options]${C_RESET}"
    echo -e "${C_DIM}       ${C_WHITE}bennett <action> <project> <env> [options]${C_RESET}"
    echo ""
    echo "Projects:"
    echo -e "  ${C_CYAN}bennett-studio${C_RESET}    Bennett Studio (desktop + web + engine)"
    echo -e "  ${C_GREEN}oshocks${C_RESET}           Oshocks project"
    echo ""
    echo "Environments:"
    echo -e "  ${C_YELLOW}dev${C_RESET}               Development mode"
    echo -e "  ${C_DIM}prod${C_RESET}              Production mode (not yet implemented)"
    echo ""
    echo -e "${C_BOLD}Actions:${C_RESET}"
    echo ""
    echo -e "${C_BOLD}  +--------------------------------------------------------------+${C_RESET}"
    echo -e "${C_BOLD}  | ${C_CYAN}START / STOP${C_RESET}${C_BOLD}                                                 |${C_RESET}"
    echo -e "${C_BOLD}  +--------------------------------------------------------------+${C_RESET}"
    echo -e "  | ${C_GREEN}start${C_RESET}                    Fast start (~2-130s)                |"
    echo -e "  |                          Builds engine only if binary missing|"
    echo -e "  |                                                              |"
    echo -e "  | ${C_GREEN}restart${C_RESET}                  Stop + start (~2-130s)              |"
    echo -e "  |                          Preserves binary, prompts for Docker|"
    echo -e "  |                                                              |"
    echo -e "  | ${C_GREEN}restart --with-docker${C_RESET}    Restart + restart Docker containers |"
    echo -e "  |                                                              |"
    echo -e "  | ${C_YELLOW}rebuild${C_RESET}                  Full clean rebuild (~6-10min)       |"
    echo -e "  |                          cargo clean + build + start         |"
    echo -e "  |                                                              |"
    echo -e "  | ${C_YELLOW}build${C_RESET}                    Compile engine only (~2-5min)      |"
    echo -e "  |                          Does not start servers              |"
    echo -e "  |                                                              |"
    echo -e "  | ${C_RED}stop${C_RESET}                     Stop all servers + Docker (optional) |"
    echo -e "${C_BOLD}  +--------------------------------------------------------------+${C_RESET}"
    echo ""
    echo -e "${C_BOLD}  +--------------------------------------------------------------+${C_RESET}"
    echo -e "${C_BOLD}  | ${C_CYAN}CLEAR / CLEAN${C_RESET}${C_BOLD}                                                |${C_RESET}"
    echo -e "${C_BOLD}  +--------------------------------------------------------------+${C_RESET}"
    echo -e "  | ${C_GREEN}clear${C_RESET}                    Clear logs only (preserves binary)  |"
    echo -e "  |                          Next start is fast                  |"
    echo -e "  |                                                              |"
    echo -e "  | ${C_RED}clear-all${C_RESET}                Clear logs + cargo clean            |"
    echo -e "  |                          ${C_RED}DESTRUCTIVE -- forces full rebuild${C_RESET}  |"
    echo -e "${C_BOLD}  +--------------------------------------------------------------+${C_RESET}"
    echo ""
    echo -e "${C_BOLD}  +--------------------------------------------------------------+${C_RESET}"
    echo -e "${C_BOLD}  | ${C_CYAN}MONITOR / DEBUG${C_RESET}${C_BOLD}                                              |${C_RESET}"
    echo -e "${C_BOLD}  +--------------------------------------------------------------+${C_RESET}"
    echo -e "  | ${C_BLUE}status${C_RESET}                   Show running status of all services   |"
    echo -e "  |                                                              |"
    echo -e "  | ${C_BLUE}logs [service]${C_RESET}           Tail logs (default: all)            |"
    echo -e "  |                          Services: ${C_CYAN}engine${C_RESET}, ${C_CYAN}web${C_RESET}, ${C_CYAN}desktop${C_RESET}, ${C_CYAN}docker${C_RESET}|"
    echo -e "  |                                                              |"
    echo -e "  | ${C_MAGENTA}attach${C_RESET}                   Attach to tmux session              |"
    echo -e "  |                          Ctrl+B then D to detach             |"
    echo -e "  |                                                              |"
    echo -e "  | ${C_MAGENTA}tree${C_RESET}                     Show project file tree              |"
    echo -e "${C_BOLD}  +--------------------------------------------------------------+${C_RESET}"
    echo ""
    echo -e "${C_BOLD}Examples:${C_RESET}"
    echo -e "  ${C_WHITE}msf bennett start bennett-studio dev${C_RESET}"
    echo -e "  ${C_WHITE}msf bennett restart bennett-studio dev${C_RESET}"
    echo -e "  ${C_WHITE}msf bennett restart --with-docker bennett-studio dev${C_RESET}"
    echo -e "  ${C_WHITE}msf bennett stop bennett-studio dev${C_RESET}"
    echo -e "  ${C_WHITE}msf bennett clear bennett-studio dev${C_RESET}"
    echo -e "  ${C_WHITE}msf bennett status bennett-studio dev${C_RESET}"
    echo -e "  ${C_WHITE}msf bennett logs bennett-studio dev engine${C_RESET}"
    echo -e "  ${C_WHITE}msf bennett attach bennett-studio dev${C_RESET}"
    echo ""
    echo -e "  ${C_WHITE}bennett start bennett-studio dev${C_RESET}"
    echo -e "  ${C_WHITE}bennett restart bennett-studio dev${C_RESET}"
    echo -e "  ${C_WHITE}bennett stop bennett-studio dev${C_RESET}"
    echo ""
    echo -e "${C_BOLD}Tips:${C_RESET}"
    echo -e "  ${C_DIM}* First start after clear-all or rebuild: ~6-10 minutes${C_RESET}"
    echo -e "  ${C_DIM}* Daily start with existing binary: ~2-130 seconds${C_RESET}"
    echo -e "  ${C_DIM}* Engine startup time depends on Docker container scanning${C_RESET}"
    echo -e "  ${C_DIM}* Use Ctrl+C to exit logs, Ctrl+B then D to detach from tmux${C_RESET}"
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
        echo -ne "${C_CYAN}$prompt [Y/n/Yes/No]: ${C_RESET}"
        read answer
        case "$answer" in
            [Yy]|[Yy][Ee][Ss]) return 0 ;;
            [Nn]|[Nn][Oo]) return 1 ;;
            *) echo -e "${C_YELLOW}Please enter Y, Yes, N, or No.${C_RESET}" ;;
        esac
    done
}

docker_is_running() {
    pgrep -x "dockerd" > /dev/null 2>&1
}

docker_status() {
    if docker_is_running; then
        echo -e "[${C_BLUE}DOCKER${C_RESET}]  ${C_GREEN}RUNNING${C_RESET}"
    else
        echo -e "[${C_BLUE}DOCKER${C_RESET}]  ${C_RED}STOPPED${C_RESET}"
    fi
}

check_docker_required() {
    if docker_is_running; then
        return 0
    fi

    echo ""
    echo -e "[${C_RED}ERROR${C_RESET}] ${C_RED}Docker is NOT running.${C_RESET}"
    echo ""
    echo "   The engine needs Docker to manage database containers."
    echo ""
    echo "   Quick start:"
    echo -e "   ${C_CYAN}sudo dockerd${C_RESET}"
    echo ""
    echo "   Or in background:"
    echo -e "   ${C_CYAN}tmux new-session -s docker 'sudo dockerd'${C_RESET}"
    echo ""

    echo -e "${C_YELLOW}Waiting 15 seconds for Docker to start (Ctrl+C to cancel)...${C_RESET}"
    for i in 15 14 13 12 11 10 9 8 7 6 5 4 3 2 1; do
        if docker_is_running; then
            echo ""
            echo -e "[${C_GREEN}OK${C_RESET}] ${C_GREEN}Docker detected! Proceeding...${C_RESET}"
            return 0
        fi
        echo -ne "\r  ${C_DIM}$i seconds remaining... ${C_RESET}"
        sleep 1
    done
    echo ""
    echo -e "[${C_RED}ERROR${C_RESET}] ${C_RED}Docker still not running. Start it manually, then retry.${C_RESET}"
    echo ""
    return 1
}

stop_docker() {
    if docker_is_running; then
        if ask_docker "Stop Docker?"; then
            echo -e "[${C_BLUE}DOCKER${C_RESET}] Stopping Docker daemon..."
            if tmux has-session -t docker 2>/dev/null; then
                tmux kill-session -t docker 2>/dev/null
            fi
            sudo pkill -x dockerd 2>/dev/null
            sleep 1
            echo -e "[${C_GREEN}OK${C_RESET}] ${C_GREEN}Docker daemon stopped${C_RESET}"
        else
            echo -e "[${C_BLUE}DOCKER${C_RESET}] Docker left running."
        fi
    else
        echo -e "[${C_BLUE}DOCKER${C_RESET}] Docker is not running."
    fi
}

clear_docker() {
    if ask_docker "Clear Docker logs?"; then
        rm -f /tmp/dockerd.log 2>/dev/null
        echo -e "[${C_GREEN}OK${C_RESET}] ${C_GREEN}Docker logs cleared${C_RESET}"
    else
        echo -e "[${C_BLUE}DOCKER${C_RESET}] Logs left intact."
    fi
}

# ============================================================================
# Engine Build Helpers
# ============================================================================

build_engine() {
    echo -e "[${C_YELLOW}BUILD${C_RESET}] Building Bennett Engine..."
    echo -e "   ${C_DIM}Binary: $BINARY${C_RESET}"
    echo ""

    # Kill any stale cargo processes that might hold the build lock
    pkill -f "cargo.*bennett" 2>/dev/null || true
    pkill -f "rustc.*bennett" 2>/dev/null || true
    pkill -f "rust-lld" 2>/dev/null || true
    sleep 2

    cd "$ENGINE_DIR" || exit 1
    if cargo build --bin bennett-engine 2>&1 | tee /tmp/bennett-engine-build.log; then
        echo ""
        echo -e "[${C_GREEN}OK${C_RESET}] ${C_GREEN}Engine built successfully${C_RESET}"
        return 0
    else
        echo ""
        echo -e "[${C_RED}ERROR${C_RESET}] ${C_RED}Engine build failed!${C_RESET}"
        echo -e "   ${C_DIM}Check: cat /tmp/bennett-engine-build.log${C_RESET}"
        return 1
    fi
}

# ============================================================================
# CRITICAL: Kill all engine processes to prevent SQLite lock conflicts
# ============================================================================
kill_all_engines() {
    echo -e "[${C_YELLOW}ENGINE${C_RESET}] Ensuring no engine processes are running..."

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
        echo -e "   ${C_YELLOW}Killing orphaned processes: $pids${C_RESET}"
        pkill -9 -f "bennett-engine" 2>/dev/null || true
        sleep 2
    fi

    # Verify they're dead
    if pgrep -f "bennett-engine" > /dev/null 2>&1; then
        echo -e "[${C_YELLOW}WARN${C_RESET}] ${C_YELLOW}Some engine processes still running!${C_RESET}"
        pkill -9 -f "bennett-engine" 2>/dev/null || true
        sleep 1
    fi

    echo -e "[${C_GREEN}OK${C_RESET}] ${C_GREEN}All engine processes stopped${C_RESET}"
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
          # START: Fast start -- build only if binary missing
          # ------------------------------------------------------------------
          start)
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_CYAN}STARTING Bennett Studio Dev Servers${C_RESET}${C_BOLD}                          |${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo ""

            if ! check_docker_required; then
                exit 1
            fi
            echo ""

            # Check if binary exists -- build if missing
            if [ ! -x "$BINARY" ]; then
                echo -e "[${C_YELLOW}WARN${C_RESET}] ${C_YELLOW}Engine binary not found.${C_RESET}"
                echo "   Running initial build (this takes ~10 min)..."
                echo ""
                if ! build_engine; then
                    exit 1
                fi
                echo ""
            else
                echo -e "[${C_GREEN}OK${C_RESET}] ${C_GREEN}Using pre-built engine:${C_RESET} ${C_WHITE}$BINARY${C_RESET}"
                echo ""
            fi

            if tmux has-session -t bennett-studio-dev 2>/dev/null; then
              echo -e "[${C_YELLOW}WARN${C_RESET}] ${C_YELLOW}tmux session 'bennett-studio-dev' already exists!${C_RESET}"
              echo "   Reattach: tmux attach -t bennett-studio-dev"
              echo "   Kill:     tmux kill-session -t bennett-studio-dev"
              exit 1
            fi

            echo -e "[${C_MAGENTA}TMUX${C_RESET}] Creating session 'bennett-studio-dev'..."
            tmux new-session -d -s bennett-studio-dev -n servers
            tmux send-keys -t bennett-studio-dev:0 'bash "/home/msf_bennett/studio.dev/bennett studio/scripts/tmux-bennett-start.sh"' C-m

            echo ""
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_GREEN}ALL SERVERS STARTED IN TMUX${C_RESET}${C_BOLD}                                 |${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo ""
            echo -e "  ${C_CYAN}Attach:${C_RESET} tmux attach -t bennett-studio-dev"
            echo -e "  ${C_CYAN}Detach:${C_RESET} Ctrl+B, then D"
            echo -e "  ${C_CYAN}Kill:${C_RESET}   tmux kill-session -t bennett-studio-dev"
            echo ""
            echo "  Services:"
            echo -e "    [${C_BLUE}DOCKER${C_RESET}]  $(docker_status | cut -d' ' -f2-)"
            echo -e "    [${C_YELLOW}ENGINE${C_RESET}]  ${C_CYAN}http://localhost:3000${C_RESET}"
            echo -e "    [${C_GREEN}WEB${C_RESET}]     ${C_CYAN}http://localhost:5173${C_RESET}"
            echo -e "    [${C_MAGENTA}DESKTOP${C_RESET}] ${C_CYAN}http://localhost:5174${C_RESET}"
            echo ""
            echo "  Servers survive terminal closes!"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo ""

            # Ask if user wants to attach now
            while true; do
                echo -ne "${C_CYAN}Attach to tmux session now? [Y/n]: ${C_RESET}"
                read attach_answer
                case "$attach_answer" in
                    [Yy]*|"")
                        echo -e "[${C_MAGENTA}TMUX${C_RESET}] Attaching to bennett-studio-dev..."
                        tmux attach -t bennett-studio-dev
                        break
                        ;;
                    [Nn]*)
                        echo -e "[${C_MAGENTA}TMUX${C_RESET}] Session running in background."
                        echo "   Attach later: tmux attach -t bennett-studio-dev"
                        break
                        ;;
                    *)
                        echo -e "${C_YELLOW}Please answer y or n${C_RESET}"
                        ;;
                esac
            done
            ;;

          # ------------------------------------------------------------------
          # RESTART: Fast restart -- stop + start (NO rebuild unless binary missing)
          # ------------------------------------------------------------------
          restart)
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_CYAN}RESTARTING Bennett Studio Dev Servers${C_RESET}${C_BOLD}                        |${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
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
              echo -e "[${C_MAGENTA}TMUX${C_RESET}] Stopping session..."
              tmux kill-session -t bennett-studio-dev
            fi

            # CRITICAL: Kill ALL engine processes to release SQLite lock
            kill_all_engines

            # Ask if user wants to restart Docker containers too
            echo ""
            while true; do
                echo -ne "[${C_BLUE}DOCKER${C_RESET}] ${C_CYAN}Restart database containers too? [y/N]: ${C_RESET}"
                read restart_docker
                case "$restart_docker" in
                    [Yy]*)
                        echo -e "[${C_BLUE}DOCKER${C_RESET}] Restarting Bennett containers..."
                        docker restart $(docker ps -q --filter "label=bennett-managed=true") 2>/dev/null || echo "   No containers to restart"
                        echo -e "[${C_GREEN}OK${C_RESET}] ${C_GREEN}Containers restarted${C_RESET}"
                        break
                        ;;
                    [Nn]*|"")
                        echo -e "[${C_BLUE}DOCKER${C_RESET}] Leaving containers running"
                        break
                        ;;
                    *)
                        echo -e "${C_YELLOW}Please answer y or n${C_RESET}"
                        ;;
                esac
            done
            echo ""

            echo -e "[${C_GREEN}WEB${C_RESET}] Stopping Web..."
            "$PROJECT_DIR/scripts/web-dev-control" stop 2>/dev/null || true
            echo -e "[${C_MAGENTA}DESKTOP${C_RESET}] Stopping Desktop..."
            "$PROJECT_DIR/scripts/desktop-dev-control" stop 2>/dev/null || true
            echo ""

            # Only rebuild if binary is missing (e.g., after cargo clean)
            if [ ! -x "$BINARY" ]; then
                echo -e "[${C_YELLOW}WARN${C_RESET}] ${C_YELLOW}Engine binary missing. Rebuilding...${C_RESET}"
                if ! build_engine; then
                    exit 1
                fi
                echo ""
            else
                echo -e "[${C_GREEN}OK${C_RESET}] ${C_GREEN}Using existing binary:${C_RESET} ${C_WHITE}$BINARY${C_RESET}"
                echo -e "   ${C_DIM}(Run 'msf bennett rebuild bennett-studio dev' for full clean rebuild)${C_RESET}"
                echo ""
            fi

            # Start fresh
            echo -e "[${C_GREEN}START${C_RESET}] Starting servers..."
            "$0" bennett start bennett-studio dev
            ;;

          # ------------------------------------------------------------------
          # REBUILD: Full clean rebuild -- use when dependencies change
          # ------------------------------------------------------------------
          rebuild)
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_RED}FULL REBUILD Bennett Studio Dev${C_RESET}${C_BOLD}                              |${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo ""

            # Kill tmux session
            if tmux has-session -t bennett-studio-dev 2>/dev/null; then
              echo -e "[${C_MAGENTA}TMUX${C_RESET}] Stopping session..."
              tmux kill-session -t bennett-studio-dev
            fi

            # CRITICAL: Kill ALL engine processes
            kill_all_engines

            "$PROJECT_DIR/scripts/web-dev-control" stop 2>/dev/null || true
            "$PROJECT_DIR/scripts/desktop-dev-control" stop 2>/dev/null || true
            echo ""

            # Clean and rebuild
            echo -e "[${C_YELLOW}CLEAN${C_RESET}] Running cargo clean..."
            cd "$PROJECT_DIR" || exit 1
            cargo clean
            echo ""

            echo -e "[${C_YELLOW}BUILD${C_RESET}] Building engine from scratch..."
            if ! build_engine; then
                exit 1
            fi
            echo ""

            # Start fresh
            echo -e "[${C_GREEN}START${C_RESET}] Starting servers..."
            "$0" bennett start bennett-studio dev
            ;;

          # ------------------------------------------------------------------
          # BUILD: Compile engine only, don't start servers
          # ------------------------------------------------------------------
          build)
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_YELLOW}BUILDING Bennett Studio Engine${C_RESET}${C_BOLD}                                 |${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo ""

            if ! check_docker_required; then
                exit 1
            fi
            echo ""

            if ! build_engine; then
                exit 1
            fi
            echo ""
            echo -e "[${C_GREEN}OK${C_RESET}] ${C_GREEN}Build complete.${C_RESET} Start servers with:"
            echo -e "   ${C_WHITE}msf bennett start bennett-studio dev${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            ;;

          # ------------------------------------------------------------------
          # STOP
          # ------------------------------------------------------------------
          stop)
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_RED}STOPPING Bennett Studio Dev Servers${C_RESET}${C_BOLD}                          |${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo ""

            if tmux has-session -t bennett-studio-dev 2>/dev/null; then
              echo -e "[${C_MAGENTA}TMUX${C_RESET}] Killing session..."
              tmux kill-session -t bennett-studio-dev
            fi

            # CRITICAL: Kill ALL engine processes
            kill_all_engines

            echo -e "[${C_GREEN}WEB${C_RESET}] Stopping Web..."
            "$PROJECT_DIR/scripts/web-dev-control" stop 2>/dev/null || true
            echo ""

            echo -e "[${C_MAGENTA}DESKTOP${C_RESET}] Stopping Desktop..."
            "$PROJECT_DIR/scripts/desktop-dev-control" stop 2>/dev/null || true
            echo ""

            stop_docker
            echo ""

            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_GREEN}ALL SERVERS STOPPED${C_RESET}${C_BOLD}                                          |${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            ;;

          # ------------------------------------------------------------------
          # CLEAR: Clear logs only (does NOT destroy binary)
          # ------------------------------------------------------------------
          clear)
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_CYAN}CLEARING Bennett Studio Dev Logs${C_RESET}${C_BOLD}                             |${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo ""

            clear_docker
            echo ""

            echo -e "[${C_CYAN}CLEAR${C_RESET}] Clearing Engine logs..."
            "$PROJECT_DIR/scripts/engine-control" clear 2>/dev/null || echo "Engine clear not available"
            echo ""

            echo -e "[${C_CYAN}CLEAR${C_RESET}] Clearing Web logs..."
            "$PROJECT_DIR/scripts/web-dev-control" clear
            echo ""

            echo -e "[${C_CYAN}CLEAR${C_RESET}] Clearing Desktop logs..."
            "$PROJECT_DIR/scripts/desktop-dev-control" clear
            echo ""

            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_GREEN}ALL LOGS CLEARED${C_RESET}${C_BOLD}                                             |${C_RESET}"
            echo -e "   ${C_DIM}Binary preserved: $BINARY${C_RESET}"
            echo -e "   ${C_DIM}Next start will be fast (~2 seconds)${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo ""
            echo -e "   To also clear build cache (forces full rebuild):"
            echo -e "   ${C_WHITE}msf bennett clear-all bennett-studio dev${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            ;;

          # ------------------------------------------------------------------
          # CLEAR-ALL: Clear logs + cargo clean (DESTRUCTIVE -- forces rebuild)
          # ------------------------------------------------------------------
          clear-all)
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_RED}CLEARING ALL -- Logs + Build Cache${C_RESET}${C_BOLD}                           |${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo ""
            echo -e "[${C_YELLOW}WARN${C_RESET}] ${C_RED}This will delete the compiled binary!${C_RESET}"
            echo -e "   ${C_DIM}Next start will require a full rebuild (~6-10 minutes).${C_RESET}"
            echo ""

            # Run regular clear first
            clear_docker
            echo ""

            echo -e "[${C_CYAN}CLEAR${C_RESET}] Clearing Engine logs..."
            "$PROJECT_DIR/scripts/engine-control" clear 2>/dev/null || true
            echo ""

            echo -e "[${C_CYAN}CLEAR${C_RESET}] Clearing Web logs..."
            "$PROJECT_DIR/scripts/web-dev-control" clear
            echo ""

            echo -e "[${C_CYAN}CLEAR${C_RESET}] Clearing Desktop logs..."
            "$PROJECT_DIR/scripts/desktop-dev-control" clear
            echo ""

            echo -e "[${C_YELLOW}CLEAN${C_RESET}] Clearing cargo build cache..."
            cd "$PROJECT_DIR" || exit 1
            cargo clean 2>/dev/null || echo "Cargo clean not available"
            echo ""

            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_GREEN}ALL LOGS AND BUILD CACHE CLEARED${C_RESET}${C_BOLD}                             |${C_RESET}"
            echo -e "   ${C_DIM}Binary deleted: $BINARY${C_RESET}"
            echo -e "   ${C_DIM}Next start will require full rebuild (~6-10 minutes)${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo ""
            echo -e "   To rebuild and start:"
            echo -e "   ${C_WHITE}msf bennett rebuild bennett-studio dev${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            ;;

          # ------------------------------------------------------------------
          # STATUS
          # ------------------------------------------------------------------
          status)
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_BLUE}Bennett Studio Dev Status${C_RESET}${C_BOLD}                                    |${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo ""
            docker_status
            "$PROJECT_DIR/scripts/engine-control" status
            "$PROJECT_DIR/scripts/web-dev-control" status
            "$PROJECT_DIR/scripts/desktop-dev-control" status
            echo ""
            if [ -x "$BINARY" ]; then
                echo -e "[${C_GREEN}OK${C_RESET}] ${C_GREEN}Engine binary:${C_RESET} ${C_WHITE}$BINARY${C_RESET}"
            else
                echo -e "[${C_YELLOW}WARN${C_RESET}] ${C_YELLOW}Engine binary not found${C_RESET} -- run: ${C_WHITE}msf bennett build bennett-studio dev${C_RESET}"
            fi
            echo ""
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            ;;

          # ------------------------------------------------------------------
          # LOGS: Tail logs with optional service filter
          # Usage: msf bennett logs bennett-studio dev [engine|web|desktop|docker|all]
          # ------------------------------------------------------------------
          logs)
            LOG_SERVICE="${5:-all}"

            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_BLUE}TAILING Bennett Studio Dev Logs${C_RESET}${C_BOLD}                              |${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo ""

            case "$LOG_SERVICE" in
              engine|e)
                echo -e "[${C_YELLOW}ENGINE${C_RESET}] Tailing Engine log..."
                echo -e "   ${C_DIM}Press Ctrl+C to stop${C_RESET}"
                echo ""
                tail -f /tmp/bennett-engine.log 2>/dev/null || echo -e "[${C_RED}ERROR${C_RESET}] Engine log not found"
                ;;
              web|w)
                echo -e "[${C_GREEN}WEB${C_RESET}] Tailing Web log..."
                echo -e "   ${C_DIM}Press Ctrl+C to stop${C_RESET}"
                echo ""
                tail -f /tmp/bennett-web.log 2>/dev/null || echo -e "[${C_RED}ERROR${C_RESET}] Web log not found"
                ;;
              desktop|d)
                echo -e "[${C_MAGENTA}DESKTOP${C_RESET}] Tailing Desktop log..."
                echo -e "   ${C_DIM}Press Ctrl+C to stop${C_RESET}"
                echo ""
                tail -f /tmp/bennett-desktop.log 2>/dev/null || echo -e "[${C_RED}ERROR${C_RESET}] Desktop log not found"
                ;;
              docker|dockerd|doc)
                echo -e "[${C_BLUE}DOCKER${C_RESET}] Tailing Docker log..."
                echo -e "   ${C_DIM}Press Ctrl+C to stop${C_RESET}"
                echo ""
                tail -f /tmp/dockerd.log 2>/dev/null || echo -e "[${C_RED}ERROR${C_RESET}] Docker log not found"
                ;;
              all|a|*)
                echo -e "[${C_WHITE}ALL${C_RESET}] Tailing ALL logs..."
                echo -e "   ${C_DIM}Press Ctrl+C to stop${C_RESET}"
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
              echo -e "[${C_MAGENTA}TMUX${C_RESET}] Attaching to bennett-studio-dev session..."
              tmux attach -t bennett-studio-dev
            else
              echo -e "[${C_RED}ERROR${C_RESET}] No tmux session 'bennett-studio-dev' found."
              echo "   Start servers first: msf bennett start bennett-studio dev"
              exit 1
            fi
            ;;

          # ------------------------------------------------------------------
          # TREE
          # ------------------------------------------------------------------
          tree)
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
            echo -e "${C_BOLD}|  ${C_MAGENTA}Bennett Studio Project Tree${C_RESET}${C_BOLD}                                  |${C_RESET}"
            echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
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
            echo -e "[${C_RED}ERROR${C_RESET}] ${C_RED}Unknown action: '$ACTION'${C_RESET}"
            echo ""
            show_help
            exit 1
            ;;
        esac
        ;;
      *)
        echo -e "${C_RED}Environment '$ENV' not found for bennett-studio. Use 'dev'.${C_RESET}"
        exit 1
        ;;
    esac
    ;;
  *)
    echo -e "${C_RED}Project '$PROJECT' not found. Available: oshocks, bennett-studio${C_RESET}"
    exit 1
    ;;
esac
