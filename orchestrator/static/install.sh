#!/bin/bash
set -euo pipefail

# cocompute host installer
# Usage: curl -sSf https://host/static/install.sh | COCOMPUTE_URL=https://host bash -s -- --token TOKEN

TOKEN=""
BASE_URL="${COCOMPUTE_URL:-}"
OLLAMA_URL=""
OLLAMA_PORT=""
INSTALL_DIR="$HOME/.cocompute"

while [[ $# -gt 0 ]]; do
    case $1 in
        --token) TOKEN="$2"; shift 2 ;;
        --ollama-url) OLLAMA_URL="$2"; shift 2 ;;
        --ollama-port) OLLAMA_PORT="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [ -z "$TOKEN" ]; then
    echo "Usage: curl -sSf <url>/install.sh | bash -s -- --token TOKEN"
    exit 1
fi

if [ -z "$BASE_URL" ]; then
    echo "Error: BASE_URL not set. Fetch this script from the orchestrator: curl -sSf <url>/install.sh"
    exit 1
fi

# Detect platform
ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$ARCH" in
    x86_64)  PLATFORM="${OS}-x86_64" ;;
    aarch64) PLATFORM="${OS}-arm64" ;;
    arm64)   PLATFORM="${OS}-arm64" ;;
    *)       echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Fetch orchestrator endpoint ID
echo "Fetching orchestrator info from $BASE_URL..."
NODE_INFO=$(curl -sSf "$BASE_URL/v1/node-info")
ORCHESTRATOR=$(echo "$NODE_INFO" | grep -o '"endpoint_id":"[^"]*"' | cut -d'"' -f4)
if [ -z "$ORCHESTRATOR" ]; then
    echo "Error: could not fetch orchestrator endpoint ID"
    exit 1
fi

install_systemd() {
    local SERVICE_DIR="$HOME/.config/systemd/user"
    mkdir -p "$SERVICE_DIR"

    cat > "$SERVICE_DIR/cocompute-host.service" <<SERVICEEOF
[Unit]
Description=cocompute host
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/cocompute-host --orchestrator-url $BASE_URL --setup-token $TOKEN$EXTRA_ARGS
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
SERVICEEOF

    systemctl --user daemon-reload
    systemctl --user enable --now cocompute-host.service

    echo ""
    echo "Service installed and started."
    echo "  Status:  systemctl --user status cocompute-host"
    echo "  Logs:    journalctl --user -u cocompute-host -f"
    echo "  Stop:    systemctl --user stop cocompute-host"
    echo "  Restart: systemctl --user restart cocompute-host"
}

install_launchd() {
    local PLIST_DIR="$HOME/Library/LaunchAgents"
    local PLIST="$PLIST_DIR/ai.cocompute.host.plist"
    local LOG_DIR="$INSTALL_DIR/logs"
    mkdir -p "$PLIST_DIR" "$LOG_DIR"

    # Build plist args
    local PLIST_EXTRA=""
    [ -n "$OLLAMA_URL" ] && PLIST_EXTRA="$PLIST_EXTRA
        <string>--ollama-url</string>
        <string>$OLLAMA_URL</string>"
    [ -n "$OLLAMA_PORT" ] && PLIST_EXTRA="$PLIST_EXTRA
        <string>--ollama-port</string>
        <string>$OLLAMA_PORT</string>"

    cat > "$PLIST" <<PLISTEOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>ai.cocompute.host</string>
    <key>ProgramArguments</key>
    <array>
        <string>$INSTALL_DIR/cocompute-host</string>
        <string>--orchestrator-url</string>
        <string>$BASE_URL</string>
        <string>--setup-token</string>
        <string>$TOKEN</string>$PLIST_EXTRA
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$LOG_DIR/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>$LOG_DIR/stderr.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>RUST_LOG</key>
        <string>info</string>
    </dict>
</dict>
</plist>
PLISTEOF

    # Unload if already running
    launchctl bootout "gui/$(id -u)/ai.cocompute.host" 2>/dev/null || true

    launchctl bootstrap "gui/$(id -u)" "$PLIST"

    echo ""
    echo "Service installed and started."
    echo "  Status:  launchctl print gui/$(id -u)/ai.cocompute.host"
    echo "  Logs:    tail -f $LOG_DIR/stderr.log"
    echo "  Stop:    launchctl bootout gui/$(id -u)/ai.cocompute.host"
    echo "  Restart: launchctl kickstart -k gui/$(id -u)/ai.cocompute.host"
}

# Build extra args for ollama config
EXTRA_ARGS=""
[ -n "$OLLAMA_URL" ] && EXTRA_ARGS="$EXTRA_ARGS --ollama-url $OLLAMA_URL"
[ -n "$OLLAMA_PORT" ] && EXTRA_ARGS="$EXTRA_ARGS --ollama-port $OLLAMA_PORT"

# Main install flow
echo ""
echo "cocompute host installer"
echo "  Platform:      $PLATFORM"
echo "  Orchestrator:  $ORCHESTRATOR"
echo "  Install dir:   $INSTALL_DIR"
[ -n "$OLLAMA_URL" ] && echo "  Ollama URL:    $OLLAMA_URL"
[ -n "$OLLAMA_PORT" ] && echo "  Ollama port:   $OLLAMA_PORT"
echo ""

mkdir -p "$INSTALL_DIR"

echo "Downloading cocompute-host for $PLATFORM..."
curl -sSf "$BASE_URL/v1/update/$PLATFORM" -o "$INSTALL_DIR/cocompute-host"
chmod +x "$INSTALL_DIR/cocompute-host"
echo "  Downloaded to $INSTALL_DIR/cocompute-host"

cat > "$INSTALL_DIR/config.toml" <<CONFEOF
orchestrator_url = "$BASE_URL"
setup_token = "$TOKEN"
CONFEOF
echo "  Config written to $INSTALL_DIR/config.toml"

echo ""
if [ "$OS" = "linux" ]; then
    install_systemd
elif [ "$OS" = "darwin" ]; then
    install_launchd
else
    echo "Unknown OS ($OS). Skipping service install."
    echo "Run manually: $INSTALL_DIR/cocompute-host --orchestrator-url $BASE_URL --setup-token $TOKEN$EXTRA_ARGS"
fi
