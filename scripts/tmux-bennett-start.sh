#!/bin/bash

exec < /dev/null

C_RESET='\033[0m'
C_RED='\033[0;31m'
C_GREEN='\033[0;32m'
C_YELLOW='\033[0;33m'
C_BLUE='\033[0;34m'
C_MAGENTA='\033[0;35m'
C_CYAN='\033[0;36m'
C_BOLD='\033[1m'
C_DIM='\033[2m'

PROJECT_DIR="/home/msf_bennett/studio.dev/bennett studio"

# ============================================================================
# Browser helper: reload existing tab or open new one
# ============================================================================
reload_or_open_tab() {
    local url="$1"
    local title_pattern="$2"
    
    # Check if running in WSL
    if grep -qEi "(Microsoft|WSL)" /proc/version 2>/dev/null; then
        local ps_cmd='
            $url = "'$url'"
            $shell = New-Object -ComObject Shell.Application
            $found = $false
            foreach ($window in $shell.Windows()) {
                if ($window.LocationURL -eq $url) {
                    $window.Refresh()
                    $found = $true
                    break
                }
            }
            if (-not $found) {
                Start-Process $url
            }
        '
        if command -v powershell.exe >/dev/null 2>&1; then
            powershell.exe -Command "$ps_cmd" >/dev/null 2>&1 &
            echo -e "[${C_GREEN}BROWSER${C_RESET}] ${C_GREEN}Reloaded or opened:${C_RESET} ${C_CYAN}$url${C_RESET}"
            return 0
        fi
    fi
    
    # Linux native: try xdotool first
    if command -v xdotool >/dev/null 2>&1; then
        local win_id=$(xdotool search --name "$title_pattern" 2>/dev/null | head -1)
        if [ -n "$win_id" ]; then
            xdotool windowactivate "$win_id" 2>/dev/null
            xdotool key --window "$win_id" F5 2>/dev/null
            echo -e "[${C_GREEN}BROWSER${C_RESET}] ${C_GREEN}Reloaded existing tab:${C_RESET} ${C_CYAN}$url${C_RESET}"
            return 0
        fi
    fi
    
    # Fallback: just open the URL
    if command -v xdg-open >/dev/null 2>&1; then
        xdg-open "$url" >/dev/null 2>&1 &
    elif command -v cmd.exe >/dev/null 2>&1; then
        cmd.exe /c start "$url" >/dev/null 2>&1 &
    fi
    echo -e "[${C_GREEN}BROWSER${C_RESET}] ${C_GREEN}Opened new tab:${C_RESET} ${C_CYAN}$url${C_RESET}"
}

echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
echo -e "${C_BOLD}|  ${C_GREEN}STARTING Bennett Studio Dev Servers${C_RESET}${C_BOLD}                          |${C_RESET}"
echo -e "${C_BOLD}+--------------------------------------------------------------+${C_RESET}"
echo ""

echo -e "[${C_YELLOW}CLEANUP${C_RESET}] Cleaning up zombie processes..."
fuser -k 5173/tcp 2>/dev/null || true
fuser -k 5174/tcp 2>/dev/null || true
pkill -f "vite.*port 517[34]" 2>/dev/null || true
pkill -f "tauri.*dev" 2>/dev/null || true
sleep 2

echo -e "[${C_YELLOW}ENGINE${C_RESET}] Starting Engine..."
"$PROJECT_DIR/scripts/engine-control" start

# Read the ACTUAL port the engine bound to
ENGINE_PORT=$(cat /tmp/bennett-engine.port 2>/dev/null || echo "3001")

echo ""
echo -e "[${C_CYAN}WAIT${C_RESET}] Waiting for health check on port ${C_CYAN}$ENGINE_PORT${C_RESET}..."
echo -e "   ${C_DIM}(Engine may take 2-3 minutes to scan Docker containers on first start)${C_RESET}"
for i in {1..1440}; do
    if curl -s "http://localhost:$ENGINE_PORT/api/health" > /dev/null 2>&1; then
        echo -e "[${C_GREEN}OK${C_RESET}] ${C_GREEN}Engine ready on ${C_CYAN}http://localhost:$ENGINE_PORT${C_RESET}"
        break
    fi
    sleep 1
    if [ $i -eq 60 ]; then
        echo -e "   [${C_YELLOW}TIME${C_RESET}] 1 minute elapsed, still waiting..."
    fi
    if [ $i -eq 120 ]; then
        echo -e "   [${C_YELLOW}TIME${C_RESET}] 2 minutes elapsed, still waiting..."
    fi
    if [ $i -eq 300 ]; then
        echo -e "   [${C_YELLOW}TIME${C_RESET}] 5 minutes elapsed, still waiting..."
    fi
    if [ $i -eq 600 ]; then
        echo -e "   [${C_YELLOW}TIME${C_RESET}] 10 minutes elapsed, still waiting..."
    fi
    if [ $i -eq 900 ]; then
        echo -e "   [${C_YELLOW}TIME${C_RESET}] 15 minutes elapsed, still waiting..."
    fi
    if [ $i -eq 1200 ]; then
        echo -e "   [${C_YELLOW}TIME${C_RESET}] 20 minutes elapsed, still waiting..."
    fi
    if [ $i -eq 1440 ]; then
        echo -e "[${C_RED}ERROR${C_RESET}] Engine health check failed after 24 minutes."
        echo -e "   ${C_DIM}Check the log for hangs:${C_RESET}"
        echo -e "   ${C_WHITE}tail -50 /tmp/bennett-engine.log${C_RESET}"
        echo ""
        echo -e "   ${C_YELLOW}Common causes:${C_RESET}"
        echo -e "   - Docker container scanning is slow (many containers)"
        echo -e "   - SQLite database is locked by another engine process"
        echo -e "   - Port conflict (another process on ${C_CYAN}$ENGINE_PORT${C_RESET})"
        echo -e "   - Engine binary is corrupted (try: ${C_WHITE}msf bennett rebuild bennett-studio dev${C_RESET})"
        exit 1
    fi
done
echo ""

echo -e "[${C_GREEN}WEB${C_RESET}] Starting Web..."
"$PROJECT_DIR/scripts/web-dev-control" start
sleep 2

# Reload or open Web tab
reload_or_open_tab "http://localhost:5173" "localhost:5173"

echo ""

echo -e "[${C_MAGENTA}DESKTOP${C_RESET}] Starting Desktop..."
cd "$PROJECT_DIR/desktop" || exit 1

# For desktop, we run in foreground in tmux, but we can reload the web view
# The desktop app itself handles its own window
npm run tauri dev

echo ""
echo -e "[${C_RED}STOP${C_RESET}] Desktop stopped. Cleaning up..."
"$PROJECT_DIR/scripts/web-dev-control" stop 2>/dev/null || true
"$PROJECT_DIR/scripts/engine-control" stop 2>/dev/null || true
echo -e "[${C_GREEN}OK${C_RESET}] ${C_GREEN}All services stopped.${C_RESET}"
