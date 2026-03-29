#!/usr/bin/env bash
set -euo pipefail

# Periodically resize Chromium window to match root window size.
# Requires: xdotool, xdpyinfo

DISPLAY=${DISPLAY:-:1}
export DISPLAY

get_root_size() {
  xdpyinfo 2>/dev/null | awk '/dimensions:/{print $2}' | sed 's/x/ /'
}

resize_chrome() {
  local w="$1" h="$2"
  # Find a Chromium/Chrome window; prefer visible ones
  local win
  win=$(xdotool search --onlyvisible --class Chromium 2>/dev/null | head -n1 || true)
  if [ -z "$win" ]; then
    win=$(xdotool search --onlyvisible --class chrome 2>/dev/null | head -n1 || true)
  fi
  if [ -n "$win" ]; then
    xdotool windowmove "$win" 0 0 || true
    xdotool windowsize "$win" "$w" "$h" || true
  fi
}

# Wait until X is up
for i in $(seq 1 60); do
  if xdpyinfo >/dev/null 2>&1; then break; fi
  sleep 0.25
done

prev_w=0
prev_h=0
while true; do
  read -r w h < <(get_root_size || echo "0 0")
  if [ "${w:-0}" -gt 0 ] && [ "${h:-0}" -gt 0 ]; then
    if [ "$w" != "$prev_w" ] || [ "$h" != "$prev_h" ]; then
      resize_chrome "$w" "$h"
      prev_w=$w; prev_h=$h
    fi
  fi
  sleep 0.5
done

