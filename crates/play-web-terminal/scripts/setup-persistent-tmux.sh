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
    
    # Clean up any existing installation
    rm -rf "$HOME/.tmux/plugins"
    
    # Create plugin directory
    mkdir -p "$HOME/.tmux/plugins"
    
    # Install tmux plugin manager (TPM)
    echo "Installing tmux plugin manager..."
    git clone https://github.com/tmux-plugins/tpm "$HOME/.tmux/plugins/tpm"
    
    # Create tmux config with resurrect plugin
    cat > "$HOME/.tmux.conf" << 'EOF'
# tmux configuration for persistent sessions

# Set plugin path
set-environment -g TMUX_PLUGIN_MANAGER_PATH '~/.tmux/plugins/'

# Set base directory for tmux-resurrect saves
set -g @resurrect-dir '/root/.local/share/play/.tmux-resurrect'

# Enable automatic session save/restore
set -g @resurrect-save 'S'
set -g @resurrect-restore 'R'

# Save pane contents
set -g @resurrect-capture-pane-contents 'on'

# Restore more programs (including ping and other network tools)
set -g @resurrect-processes ':all:'

# Enable restoration of shell commands
set -g @resurrect-save-shell-history 'on'

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

# Initialize TMUX plugin manager (keep this at the bottom of tmux.conf)
run '~/.tmux/plugins/tpm/tpm'
EOF
    
    # Manually install the plugins since install_plugins might not work
    echo "Installing tmux-resurrect plugin..."
    git clone https://github.com/tmux-plugins/tmux-resurrect "$HOME/.tmux/plugins/tmux-resurrect"
    
    echo "Installing tmux-continuum plugin..."
    git clone https://github.com/tmux-plugins/tmux-continuum "$HOME/.tmux/plugins/tmux-continuum"
    
    # Source the config in any running tmux sessions
    tmux source-file "$HOME/.tmux.conf" 2>/dev/null || true
    
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
ExecStartPost=/usr/local/bin/tmux-restore-sessions

# Save sessions before stopping
ExecStop=/usr/local/bin/tmux-save-sessions
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
# Manually save all tmux sessions with complete state

export TMUX_TMPDIR="${TMUX_TMPDIR:-/root/.local/share/play/.tmux}"
RESURRECT_DIR="/root/.local/share/play/.tmux-resurrect"
TIMESTAMP=$(date +%Y%m%d%H%M%S)

echo "Saving tmux sessions..."

# Create resurrect directory if it doesn't exist
mkdir -p "$RESURRECT_DIR"

# Method 1: Try to use tmux-resurrect if available
if [ -f "$HOME/.tmux/plugins/tmux-resurrect/scripts/save.sh" ]; then
    echo "Using tmux-resurrect to save sessions..."
    
    # Create a temporary tmux session to run the save command
    tmux new-session -d -s resurrect-save-helper 2>/dev/null || true
    
    # Send the resurrect save key binding to trigger save
    tmux send-keys -t resurrect-save-helper C-b S
    sleep 2
    
    # Kill the helper session
    tmux kill-session -t resurrect-save-helper 2>/dev/null || true
    
    echo "Sessions saved via tmux-resurrect"
fi

# Method 2: Enhanced manual save with full state
echo "Creating comprehensive backup..."

# Save detailed session information
SAVE_FILE="$RESURRECT_DIR/manual_save_${TIMESTAMP}.txt"

# Get all sessions with details
tmux list-sessions -F "SESSION:#{session_name}:#{session_windows}:#{session_created}" > "$SAVE_FILE"

# For each session, save windows and panes information
for session in $(tmux list-sessions -F "#{session_name}"); do
    echo "Saving session: $session"
    
    # Save windows for this session
    tmux list-windows -t "$session" -F "WINDOW:$session:#{window_index}:#{window_name}:#{window_layout}:#{pane_current_path}" >> "$SAVE_FILE"
    
    # Save panes for each window
    for window in $(tmux list-windows -t "$session" -F "#{window_index}"); do
        # Get pane information including working directory and running command
        tmux list-panes -t "$session:$window" -F "PANE:$session:$window:#{pane_index}:#{pane_current_path}:#{pane_current_command}:#{pane_pid}" >> "$SAVE_FILE"
        
        # Save pane contents
        tmux capture-pane -t "$session:$window" -p -S -3000 > "$RESURRECT_DIR/pane_${session}_${window}_${TIMESTAMP}.txt" 2>/dev/null || true
        
        # Try to get the full command line from /proc if available
        for pane_pid in $(tmux list-panes -t "$session:$window" -F "#{pane_pid}"); do
            if [ -f "/proc/$pane_pid/cmdline" ]; then
                echo "CMDLINE:$session:$window:$pane_pid:$(tr '\0' ' ' < /proc/$pane_pid/cmdline)" >> "$SAVE_FILE"
            fi
        done
    done
done

# Create a symlink to the latest save
ln -sf "manual_save_${TIMESTAMP}.txt" "$RESURRECT_DIR/last"

echo "Sessions saved to $RESURRECT_DIR"
echo "Save file: $SAVE_FILE"

# Also trigger tmux-continuum save if available
if tmux show-option -gqv @continuum-save-interval >/dev/null 2>&1; then
    tmux run-shell "$HOME/.tmux/plugins/tmux-continuum/scripts/continuum_save.sh" 2>/dev/null || true
fi
EOF
    chmod +x /usr/local/bin/tmux-save-sessions
    
    # Create manual restore script
    cat > /usr/local/bin/tmux-restore-sessions << 'EOF'
#!/bin/bash
# Manually restore tmux sessions with complete state including working directories and commands

export TMUX_TMPDIR="${TMUX_TMPDIR:-/root/.local/share/play/.tmux}"
RESURRECT_DIR="/root/.local/share/play/.tmux-resurrect"

echo "Restoring tmux sessions..."

# Method 1: Try tmux-resurrect first if available
if [ -f "$HOME/.tmux/plugins/tmux-resurrect/scripts/restore.sh" ]; then
    # Find the latest resurrect save file
    LATEST_RESURRECT=$(ls -t "$RESURRECT_DIR"/tmux_resurrect_*.txt 2>/dev/null | head -1)
    
    if [ -f "$LATEST_RESURRECT" ]; then
        echo "Found tmux-resurrect save: $LATEST_RESURRECT"
        
        # Create a temporary session to trigger restore
        tmux new-session -d -s resurrect-restore-helper 2>/dev/null || true
        
        # Send the resurrect restore key binding
        tmux send-keys -t resurrect-restore-helper C-b R
        sleep 3
        
        # Kill the helper session
        tmux kill-session -t resurrect-restore-helper 2>/dev/null || true
        
        echo "Attempted tmux-resurrect restore"
    fi
fi

# Method 2: Enhanced manual restore from our comprehensive save
SAVE_FILE="$RESURRECT_DIR/last"
if [ -L "$SAVE_FILE" ]; then
    # Follow symlink to actual save file
    SAVE_FILE=$(readlink -f "$SAVE_FILE")
fi

if [ ! -f "$SAVE_FILE" ]; then
    # Try to find the latest manual save
    SAVE_FILE=$(ls -t "$RESURRECT_DIR"/manual_save_*.txt 2>/dev/null | head -1)
fi

if [ -f "$SAVE_FILE" ]; then
    echo "Restoring from: $SAVE_FILE"
    
    # Parse and restore sessions
    while IFS=: read -r type name windows created rest; do
        if [ "$type" = "SESSION" ]; then
            echo "Restoring session: $name"
            # Don't recreate keeper session or already existing sessions
            if [ "$name" != "keeper" ] && ! tmux has-session -t "$name" 2>/dev/null; then
                tmux new-session -d -s "$name"
            fi
        fi
    done < "$SAVE_FILE"
    
    # Restore windows and panes with working directories
    current_session=""
    current_window=""
    
    while IFS=: read -r type param1 param2 param3 param4 param5 rest; do
        case "$type" in
            WINDOW)
                session="$param1"
                window_idx="$param2"
                window_name="$param3"
                layout="$param4"
                working_dir="$param5"
                
                if [ "$session" != "$current_session" ]; then
                    current_session="$session"
                fi
                
                # Create window with proper working directory
                if [ "$window_idx" = "0" ]; then
                    # First window already exists, just rename and cd
                    tmux rename-window -t "$session:0" "$window_name" 2>/dev/null || true
                    if [ -d "$working_dir" ]; then
                        tmux send-keys -t "$session:0" "cd '$working_dir'" C-m 2>/dev/null || true
                    fi
                else
                    # Create new window with working directory
                    if [ -d "$working_dir" ]; then
                        tmux new-window -t "$session" -n "$window_name" -c "$working_dir" 2>/dev/null || true
                    else
                        tmux new-window -t "$session" -n "$window_name" 2>/dev/null || true
                    fi
                fi
                
                current_window="$window_idx"
                ;;
                
            PANE)
                session="$param1"
                window="$param2"
                pane_idx="$param3"
                pane_dir="$param4"
                pane_cmd="$param5"
                pane_pid="$rest"
                
                # Set working directory for pane
                if [ -d "$pane_dir" ] && [ "$pane_idx" = "0" ]; then
                    tmux send-keys -t "$session:$window.$pane_idx" "cd '$pane_dir'" C-m 2>/dev/null || true
                fi
                
                # Try to restore command if it's a long-running process
                if [ -n "$pane_cmd" ] && [ "$pane_cmd" != "bash" ] && [ "$pane_cmd" != "zsh" ] && [ "$pane_cmd" != "sh" ]; then
                    echo "  Found command in pane $pane_idx: $pane_cmd"
                fi
                ;;
                
            CMDLINE)
                session="$param1"
                window="$param2"
                pid="$param3"
                cmdline="$param4 $param5 $rest"
                
                # Extract and potentially restart command
                if echo "$cmdline" | grep -qE "ping|curl|wget|ssh|telnet|nc|watch|tail -f|top|htop"; then
                    echo "  Command to restore: $cmdline"
                    # For safety, we'll just notify about these commands rather than auto-run them
                    echo "    -> To restart: tmux send-keys -t '$session:$window' '$cmdline' C-m"
                fi
                ;;
        esac
    done < "$SAVE_FILE"
    
    echo "Session structure restored"
    echo ""
    echo "Note: Long-running commands like 'ping' need to be manually restarted."
    echo "The commands that were running have been identified above."
    
    # Restore pane contents if available
    TIMESTAMP=$(basename "$SAVE_FILE" | sed 's/manual_save_\(.*\)\.txt/\1/')
    if [ -n "$TIMESTAMP" ]; then
        for pane_file in "$RESURRECT_DIR"/pane_*_"${TIMESTAMP}.txt"; do
            if [ -f "$pane_file" ]; then
                # Extract session and window from filename
                base=$(basename "$pane_file")
                parts=$(echo "$base" | sed 's/pane_\(.*\)_\(.*\)_.*\.txt/\1 \2/')
                # Could restore pane contents here if needed
                true
            fi
        done
    fi
else
    echo "No saved sessions found to restore"
    echo "Try running: tmux-save-sessions first"
fi

# List restored sessions
echo ""
echo "Current tmux sessions:"
tmux list-sessions 2>/dev/null || echo "No sessions running"
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