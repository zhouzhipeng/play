#!/bin/bash

# Simple setup for persistent tmux sessions
# This script creates a systemd service to keep tmux running across reboots

set -e

# Configuration
DATA_DIR="${DATA_DIR:-/root/.local/share/play}"
TMUX_DIR="$DATA_DIR/.tmux"
export TMUX_TMPDIR="$TMUX_DIR"

echo "=== Setting up Persistent tmux Service ==="
echo "Data directory: $DATA_DIR"
echo "Tmux socket directory: $TMUX_DIR"
echo

# Create systemd service for persistent tmux
create_systemd_service() {
    echo "Creating systemd service for tmux persistence..."

    # Create the service file
    cat > /etc/systemd/system/tmux-persistent.service << EOF
[Unit]
Description=Persistent tmux server
After=network.target

[Service]
Type=forking
User=root
Environment="TMUX_TMPDIR=$TMUX_DIR"
Environment="HOME=/root"

# Create tmux directory
ExecStartPre=/bin/mkdir -p $TMUX_DIR
ExecStartPre=/bin/chmod 700 $TMUX_DIR

# Start tmux with a keeper session to keep server alive
ExecStart=/usr/bin/tmux new-session -d -s keeper "while true; do sleep 86400; done"

# Clean shutdown
ExecStop=/usr/bin/tmux kill-server

Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable tmux-persistent.service

    echo "Systemd service created and enabled"
}

# Configure play service integration (optional)
configure_play_service() {
    echo "Configuring play service integration..."
    
    if systemctl list-unit-files | grep -q "play.service"; then
        mkdir -p /etc/systemd/system/play.service.d/
        
        cat > /etc/systemd/system/play.service.d/tmux-env.conf << EOF
[Unit]
# Ensure tmux is running before play starts
Wants=tmux-persistent.service
After=tmux-persistent.service

[Service]
# Set environment for tmux
Environment="TMUX_TMPDIR=$TMUX_DIR"
EOF
        
        systemctl daemon-reload
        echo "Play service configured"
    else
        echo "Play service not found, skipping integration"
    fi
}

# Main installation
main() {
    echo "=== Simple tmux Persistence Setup ==="
    echo
    
    # Check for tmux
    if ! command -v tmux >/dev/null 2>&1; then
        echo "ERROR: tmux is not installed!"
        echo "Install tmux first:"
        echo "  Ubuntu/Debian: apt-get install tmux"
        echo "  CentOS/RHEL: yum install tmux"
        echo "  Alpine: apk add tmux"
        exit 1
    fi
    
    # Create tmux directory
    mkdir -p "$TMUX_DIR"
    chmod 700 "$TMUX_DIR"
    
    # Install service
    create_systemd_service
    configure_play_service
    
    # Start the service
    echo
    echo "Starting tmux-persistent service..."
    systemctl start tmux-persistent.service
    
    # Verify
    if systemctl is-active --quiet tmux-persistent.service; then
        echo "✓ Service is running"
    else
        echo "✗ Service failed to start"
        echo "Check: journalctl -u tmux-persistent.service"
        exit 1
    fi
    
    # Wait a moment for tmux to fully start
    sleep 2
    
    # List sessions
    echo
    echo "Current tmux sessions:"
    tmux list-sessions 2>/dev/null || echo "No sessions found"
    
    echo
    echo "=== Setup Complete ==="
    echo
    echo "tmux server will now persist across reboots!"
    echo
    echo "The tmux socket is located at: $TMUX_DIR"
    echo "Sessions created in tmux will survive as long as the server runs."
    echo
    echo "Useful commands:"
    echo "  tmux ls                    # List sessions"
    echo "  tmux new -s mysession      # Create new session"
    echo "  tmux attach -t mysession   # Attach to session"
    echo "  systemctl status tmux-persistent"
    echo "  systemctl restart tmux-persistent"
    echo
    echo "Note: The 'keeper' session keeps tmux server alive."
    echo "      Don't kill it unless you want to stop the tmux server."
    echo
}

# Run installation
main "$@"