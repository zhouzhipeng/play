#!/usr/bin/env bash
set -euo pipefail

# Environment
: "${DISPLAY:=:1}"
: "${VNC_PORT:=5901}"
: "${NOVNC_PORT:=6901}"
: "${RESOLUTION:=1920x1080}"
: "${VNC_PW:=}"
: "${VNC_PASSWORDLESS:=true}"
: "${AUTO_START_CHROME:=true}"

export DISPLAY

mkdir -p "$HOME/.vnc"

# Configure VNC authentication
VNC_SECURITY_FLAGS=""
if [ "${VNC_PASSWORDLESS}" = "true" ] || [ "${VNC_PASSWORDLESS}" = "1" ] || [ -z "${VNC_PW}" ]; then
  # No password, explicitly disable auth
  echo "Configuring TigerVNC with no authentication (SecurityTypes=None)"
  rm -f "$HOME/.vnc/passwd" 2>/dev/null || true
  VNC_SECURITY_FLAGS="-SecurityTypes None"
else
  # Set VNC password (TigerVNC expects a special hashed format via vncpasswd -f)
  if [ ! -f "$HOME/.vnc/passwd" ]; then
    if command -v tigervncpasswd >/dev/null 2>&1; then
      tigervncpasswd -f <<<"${VNC_PW}" > "$HOME/.vnc/passwd"
      chmod 600 "$HOME/.vnc/passwd"
    elif command -v vncpasswd >/dev/null 2>&1; then
      # shellcheck disable=SC2312
      vncpasswd -f <<<"${VNC_PW}" > "$HOME/.vnc/passwd"
      chmod 600 "$HOME/.vnc/passwd"
    else
      echo "Warning: no vncpasswd tool found; consider setting VNC_PASSWORDLESS=true."
    fi
  fi
fi

# Write xstartup if missing
if [ ! -f "$HOME/.vnc/xstartup" ]; then
  cp /etc/vnc_xstartup "$HOME/.vnc/xstartup"
  chmod +x "$HOME/.vnc/xstartup"
fi

# Ensure any stale locks are removed (best effort)
rm -f /tmp/.X1-lock /tmp/.X11-unix/X1 || true

echo "Starting TigerVNC on display ${DISPLAY} (${RESOLUTION}), localhost-only..."
vncserver "$DISPLAY" \
  -geometry "${RESOLUTION}" \
  -localhost yes \
  -rfbport "$VNC_PORT" \
  ${VNC_SECURITY_FLAGS} \
  -fg \
  -xstartup "$HOME/.vnc/xstartup" &
VNC_PID=$!

# Wait a moment for VNC to initialize
sleep 1

# Start Chromium after the VNC server is up (optional)
if [ "${AUTO_START_CHROME}" = "true" ] || [ "${AUTO_START_CHROME}" = "1" ]; then
  echo "Launching Chromium..."
  # Disable sandbox for container; use an isolated user-data-dir
  chromium \
    --disable-dev-shm-usage \
    --user-data-dir="$HOME/.config/chromium-profile" \
    --start-maximized \
    --no-first-run \
    --disable-infobars \
    --autoplay-policy=no-user-gesture-required \
    >/dev/null 2>&1 &
fi

# Start noVNC in HTTP mode, bind to 0.0.0.0 to support normal and host networking
echo "Starting noVNC on :${NOVNC_PORT} (HTTP), proxying VNC localhost:${VNC_PORT}..."
novnc_proxy \
  --listen 0.0.0.0:"${NOVNC_PORT}" \
  --vnc localhost:"${VNC_PORT}"

# noVNC runs in foreground; ensure VNC is cleaned on exit
trap 'echo "Stopping VNC..."; vncserver -kill "$DISPLAY" >/dev/null 2>&1 || true' EXIT
wait "$VNC_PID" || true
