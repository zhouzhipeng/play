FROM debian:bookworm-slim

# Minimal, HTTP-only noVNC on 6901; no 5901 exposed and works with --network host

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    TZ=UTC \
    DISPLAY=:1 \
    VNC_PORT=5901 \
    NOVNC_PORT=6901 \
    RESOLUTION=1920x1080 \
    VNC_PW=vncpassword \
    AUTO_START_CHROME=true

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
      tigervnc-standalone-server tigervnc-common tigervnc-tools \
      fluxbox xterm \
      novnc websockify python3 \
      chromium ca-certificates fonts-dejavu-core \
      curl procps supervisor \
    ; \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -s /bin/bash app && \
    mkdir -p /home/app/.vnc /opt/novnc && \
    chown -R app:app /home/app /opt/novnc

# Basic VNC xstartup to launch a minimal WM
RUN printf '#!/bin/sh\n[ -x /usr/bin/fluxbox ] && exec /usr/bin/fluxbox\nexec /usr/bin/xterm\n' > /etc/vnc_xstartup && \
    chmod +x /etc/vnc_xstartup

# noVNC helper symlink for convenience
RUN ln -s /usr/share/novnc/utils/novnc_proxy /usr/local/bin/novnc_proxy

# Startup script: launches TigerVNC (:1), noVNC (6901, HTTP), and Chromium
ADD start-vnc.sh /usr/local/bin/start-vnc.sh
RUN chmod +x /usr/local/bin/start-vnc.sh && chown app:app /usr/local/bin/start-vnc.sh

# Default landing page: redirect "/" to "/vnc.html?autoconnect=1"
ADD novnc-index.html /usr/share/novnc/index.html

USER app
WORKDIR /home/app

# Do not EXPOSE 5901 by design; only 6901 is relevant to noVNC
EXPOSE 6901

ENTRYPOINT ["/usr/local/bin/start-vnc.sh"]
