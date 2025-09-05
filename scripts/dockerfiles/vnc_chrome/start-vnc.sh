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

# No vncserver xstartup needed; Xtigervnc is launched directly

# Ensure any stale locks are removed (best effort)
rm -f /tmp/.X1-lock /tmp/.X11-unix/X1 || true

echo "Starting TigerVNC (Xtigervnc) on display ${DISPLAY} (${RESOLUTION}), localhost-only..."
XVNCSRV=$(command -v Xtigervnc || true)
if [ -z "${XVNCSRV}" ]; then
  XVNCSRV=$(command -v Xvnc || true)
fi
if [ -z "${XVNCSRV}" ]; then
  echo "Error: Xtigervnc/Xvnc not found." >&2
  exit 1
fi

AUTH_FLAG=""
if [ -f "$HOME/.vnc/passwd" ] && [ -n "${VNC_PW:-}" ] && [ ! "${VNC_PASSWORDLESS}" = "true" ] && [ ! "${VNC_PASSWORDLESS}" = "1" ]; then
  AUTH_FLAG="-rfbauth $HOME/.vnc/passwd"
fi

"${XVNCSRV}" "$DISPLAY" \
  -geometry "${RESOLUTION}" \
  -localhost=1 \
  -rfbport "$VNC_PORT" \
  ${VNC_SECURITY_FLAGS} \
  -depth 24 \
  -AlwaysShared \
  ${AUTH_FLAG} \
  &
VNC_PID=$!

# Wait for VNC to listen on the port to avoid connection refused
echo "Waiting for VNC to be ready on localhost:${VNC_PORT}..."
for i in $(seq 1 60); do
  if bash -lc "</dev/tcp/127.0.0.1/${VNC_PORT}" 2>/dev/null; then
    echo "VNC is ready."
    break
  fi
  if ! kill -0 "$VNC_PID" 2>/dev/null; then
    echo "VNC server process exited unexpectedly." >&2
    break
  fi
  sleep 0.5
done

# Set a neutral background color so a blank desktop isn't pure black
if command -v xsetroot >/dev/null 2>&1; then
  xsetroot -solid "#303030" || true
fi

# Start a minimal WM without titlebars/panels to ensure Chrome maps correctly
if command -v matchbox-window-manager >/dev/null 2>&1; then
  echo "Starting matchbox-window-manager (no titlebar) ..."
  matchbox-window-manager -use_titlebar no >/dev/null 2>&1 &
  sleep 0.2
fi


# Start Chromium after the VNC server is up (optional)
if [ "${AUTO_START_CHROME}" = "true" ] || [ "${AUTO_START_CHROME}" = "1" ]; then
  echo "Launching Chromium (auto-restart on exit)..."
  CHROME_BIN=""
  for c in chromium chromium-browser google-chrome-stable google-chrome; do
    if command -v "$c" >/dev/null 2>&1; then CHROME_BIN=$(command -v "$c"); break; fi
  done
  if [ -z "$CHROME_BIN" ]; then
    echo "Chromium/Chrome not found in PATH" >&2
  fi
  LOGFILE="$HOME/.chromium.log"
  (
    while true; do
      "$CHROME_BIN" \
        --no-sandbox \
        --disable-gpu \
        --ozone-platform=x11 \
        --disable-dev-shm-usage \
        --user-data-dir="$HOME/.config/chromium-profile" \
        --window-position=0,0 \
        --start-maximized \
        --no-first-run \
        --disable-infobars \
        --autoplay-policy=no-user-gesture-required \
        "$@"
      echo "Chromium exited; restarting in 1s..." >&2
      sleep 1
    done
  ) >>"$LOGFILE" 2>&1 &
fi

# Start noVNC in HTTP mode, bind to 0.0.0.0 to support normal and host networking
echo "Starting noVNC on :${NOVNC_PORT} (HTTP), proxying VNC localhost:${VNC_PORT}..."
novnc_proxy \
  --listen 0.0.0.0:"${NOVNC_PORT}" \
  --vnc 127.0.0.1:"${VNC_PORT}"

# Keep Chromium fitted to desktop size (follows noVNC resize=remote)
/usr/local/bin/x-auto-fit.sh &

# noVNC runs in foreground; ensure VNC is cleaned on exit
trap 'echo "Stopping VNC..."; kill "$VNC_PID" >/dev/null 2>&1 || true; rm -f /tmp/.X1-lock /tmp/.X11-unix/X1 2>/dev/null || true' EXIT
wait "$VNC_PID" || true
