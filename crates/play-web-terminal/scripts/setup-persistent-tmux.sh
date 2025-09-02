#!/bin/bash

# Complete setup for persistent tmux sessions across reboots
# This script sets up tmux to survive server reboots with session restoration

set -e

# Configuration
DATA_DIR="${DATA_DIR:-/root/.local/share/play}"
TMUX_DIR="$DATA_DIR/.tmux"
TMUX_RESURRECT_DIR="$DATA_DIR/.tmux-resurrect"
export TMUX_TMPDIR="$TMUX_DIR"

echo "=== Setting up Persistent tmux with Session Restoration ==="
echo "Data directory: $DATA_DIR"
echo "Tmux socket directory: $TMUX_DIR"
echo "Session backup directory: $TMUX_RESURRECT_DIR"
echo

# 1. Install tmux-resurrect plugin for session persistence
install_tmux_resurrect() {
    echo "Installing tmux-resurrect for session persistence..."
    
    # Install tmux plugin manager (TPM) first
    if [ ! -d "$HOME/.tmux/plugins/tpm" ]; then
        git clone https://github.com/tmux-plugins/tpm "$HOME/.tmux/plugins/tpm"
    fi
    
    # Create tmux config with resurrect plugin
    cat > "$HOME/.tmux.conf" << 'EOF'
# tmux configuration for persistent sessions

# Set base directory for tmux-resurrect saves
set -g @resurrect-dir '/root/.local/share/play/.tmux-resurrect'

# Enable automatic session save/restore
set -g @resurrect-save 'S'
set -g @resurrect-restore 'R'

# Save pane contents
set -g @resurrect-capture-pane-contents 'on'

# Restore more programs
set -g @resurrect-processes 'ssh vim nvim emacs man less more tail top htop'

# Strategy for vim/nvim sessions
set -g @resurrect-strategy-vim 'session'
set -g @resurrect-strategy-nvim 'session'

# Plugin list
set -g @plugin 'tmux-plugins/tpm'
set -g @plugin 'tmux-plugins/tmux-resurrect'
set -g @plugin 'tmux-plugins/tmux-continuum'

# Auto save/restore
set -g @continuum-restore 'on'
set -g @continuum-save-interval '5' # Save every 5 minutes
set -g @continuum-boot 'on' # Auto start tmux on boot

# Initialize TMUX plugin manager
run '~/.tmux/plugins/tpm/tpm'
EOF
    
    # Install plugins
    "$HOME/.tmux/plugins/tpm/bin/install_plugins"
    
    echo "tmux-resurrect installed successfully"
}

# 2. Create systemd service that restores sessions on boot
create_systemd_service() {
    echo "Creating systemd service for tmux persistence..."
    
    # Create the service file
    cat > /etc/systemd/system/tmux-persistent.service << EOF
[Unit]
Description=Persistent tmux server with session restoration
After=network.target

[Service]
Type=forking
User=root
Environment="DATA_DIR=$DATA_DIR"
Environment="TMUX_TMPDIR=$TMUX_DIR"
Environment="HOME=/root"

# Create directories
ExecStartPre=/bin/mkdir -p $TMUX_DIR
ExecStartPre=/bin/mkdir -p $TMUX_RESURRECT_DIR
ExecStartPre=/bin/chmod 700 $TMUX_DIR

# Start tmux server with keeper session
ExecStart=/bin/bash -c 'tmux new-session -d -s keeper "while true; do sleep 86400; done"'

# Wait for server to be ready
ExecStartPost=/bin/sleep 2

# Restore saved sessions if they exist
ExecStartPost=/bin/bash -c 'if [ -f $TMUX_RESURRECT_DIR/last ]; then tmux run-shell "$HOME/.tmux/plugins/tmux-resurrect/scripts/restore.sh"; fi'

# Save sessions before stopping
ExecStop=/bin/bash -c 'tmux run-shell "$HOME/.tmux/plugins/tmux-resurrect/scripts/save.sh"'
ExecStop=/usr/bin/tmux kill-server

Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF
    
    systemctl daemon-reload
    systemctl enable tmux-persistent.service
    
    echo "Systemd service created and enabled"
}

# 3. Create session save/restore scripts
create_helper_scripts() {
    echo "Creating helper scripts..."
    
    # Create manual save script
    cat > /usr/local/bin/tmux-save-sessions << 'EOF'
#!/bin/bash
# Manually save all tmux sessions

export TMUX_TMPDIR="${TMUX_TMPDIR:-/root/.local/share/play/.tmux}"
RESURRECT_DIR="/root/.local/share/play/.tmux-resurrect"

echo "Saving tmux sessions..."

# Use tmux-resurrect to save
if [ -f "$HOME/.tmux/plugins/tmux-resurrect/scripts/save.sh" ]; then
    tmux run-shell "$HOME/.tmux/plugins/tmux-resurrect/scripts/save.sh"
    echo "Sessions saved to $RESURRECT_DIR"
else
    echo "tmux-resurrect not found, using basic save..."
    mkdir -p "$RESURRECT_DIR"
    tmux list-sessions -F "#{session_name}" > "$RESURRECT_DIR/session_list.txt"
    echo "Basic session list saved"
fi
EOF
    chmod +x /usr/local/bin/tmux-save-sessions
    
    # Create manual restore script
    cat > /usr/local/bin/tmux-restore-sessions << 'EOF'
#!/bin/bash
# Manually restore tmux sessions

export TMUX_TMPDIR="${TMUX_TMPDIR:-/root/.local/share/play/.tmux}"
RESURRECT_DIR="/root/.local/share/play/.tmux-resurrect"

echo "Restoring tmux sessions..."

# Use tmux-resurrect to restore
if [ -f "$HOME/.tmux/plugins/tmux-resurrect/scripts/restore.sh" ]; then
    tmux run-shell "$HOME/.tmux/plugins/tmux-resurrect/scripts/restore.sh"
    echo "Sessions restored from $RESURRECT_DIR"
else
    echo "tmux-resurrect not found, using basic restore..."
    if [ -f "$RESURRECT_DIR/session_list.txt" ]; then
        while read session; do
            if [ -n "$session" ] && [ "$session" != "keeper" ]; then
                tmux new-session -d -s "$session" 2>/dev/null || true
            fi
        done < "$RESURRECT_DIR/session_list.txt"
        echo "Basic session list restored"
    fi
fi
EOF
    chmod +x /usr/local/bin/tmux-restore-sessions
    
    echo "Helper scripts created"
}

# 4. Create cron job for periodic saves
setup_cron_job() {
    echo "Setting up cron job for periodic session saves..."
    
    # Add cron job to save sessions every 5 minutes
    (crontab -l 2>/dev/null | grep -v "tmux-save-sessions"; echo "*/5 * * * * /usr/local/bin/tmux-save-sessions >/dev/null 2>&1") | crontab -
    
    echo "Cron job configured"
}

# 5. Configure play service to work with persistent tmux
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
Environment="DATA_DIR=$DATA_DIR"

# Save sessions before restart
ExecStopPre=/usr/local/bin/tmux-save-sessions
EOF
        
        systemctl daemon-reload
        echo "Play service configured"
    else
        echo "Play service not found, skipping integration"
    fi
}

# Main installation
main() {
    echo "=== Complete tmux Persistence Setup ==="
    echo
    
    # Check for tmux
    if ! command -v tmux >/dev/null 2>&1; then
        echo "ERROR: tmux is not installed!"
        echo "Install tmux first:"
        echo "  Ubuntu/Debian: apt-get install tmux"
        echo "  CentOS/RHEL: yum install tmux"
        exit 1
    fi
    
    # Check for git (needed for plugin installation)
    if ! command -v git >/dev/null 2>&1; then
        echo "Installing git..."
        if command -v apt-get >/dev/null 2>&1; then
            apt-get update && apt-get install -y git
        elif command -v yum >/dev/null 2>&1; then
            yum install -y git
        else
            echo "ERROR: Please install git manually"
            exit 1
        fi
    fi
    
    # Create directories
    mkdir -p "$TMUX_DIR"
    mkdir -p "$TMUX_RESURRECT_DIR"
    chmod 700 "$TMUX_DIR"
    
    # Install components
    install_tmux_resurrect
    create_systemd_service
    create_helper_scripts
    setup_cron_job
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
    
    echo
    echo "=== Setup Complete ==="
    echo
    echo "tmux sessions will now persist across reboots!"
    echo
    echo "Features enabled:"
    echo "  ✓ Automatic session save every 5 minutes"
    echo "  ✓ Session restoration on boot"
    echo "  ✓ Pane contents preservation"
    echo "  ✓ Integration with play service"
    echo
    echo "Useful commands:"
    echo "  tmux-save-sessions     # Manually save all sessions"
    echo "  tmux-restore-sessions  # Manually restore sessions"
    echo "  systemctl status tmux-persistent.service"
    echo "  journalctl -u tmux-persistent.service"
    echo
    echo "Testing persistence:"
    echo "  1. Create a test session: tmux new -s test -d 'echo Hello'"
    echo "  2. Save: tmux-save-sessions"
    echo "  3. Reboot: reboot"
    echo "  4. After reboot, check: tmux ls"
    echo "  5. Your 'test' session should be restored!"
    echo
}

# Run installation
main "$@"