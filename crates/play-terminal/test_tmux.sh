#!/bin/bash

# Test script for tmux integration in web terminal

echo "Testing tmux integration..."

# Check if tmux is installed
if ! command -v tmux &> /dev/null; then
    echo "tmux is not installed. Please install tmux to use session features."
    echo "On macOS: brew install tmux"
    echo "On Ubuntu/Debian: sudo apt-get install tmux"
    exit 1
fi

echo "tmux is available: $(tmux -V)"

# Clean up any existing test sessions
tmux kill-session -t web-terminal-test 2>/dev/null || true

# Create a test session
echo "Creating test tmux session..."
tmux new-session -d -s web-terminal-test -c "$PWD" 'echo "Test session created"; bash'

# List sessions
echo "Current tmux sessions:"
tmux list-sessions

# Send a command to the test session
echo "Sending test command to session..."
tmux send-keys -t web-terminal-test "echo 'Hello from tmux!'" Enter

# Capture pane content
sleep 1
echo "Session output:"
tmux capture-pane -t web-terminal-test -p

# Clean up
echo "Cleaning up test session..."
tmux kill-session -t web-terminal-test

echo "tmux integration test completed successfully!"