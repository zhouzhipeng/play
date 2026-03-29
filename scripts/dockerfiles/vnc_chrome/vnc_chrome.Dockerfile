FROM debian:bookworm-slim

# Minimal, HTTP-only noVNC on 6901; no 5901 exposed and works with --network host

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=en_US.UTF-8 \
    LC_ALL=en_US.UTF-8 \
    LANGUAGE=en_US:en \
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
      locales \
      tigervnc-standalone-server tigervnc-common tigervnc-tools \
      x11-xserver-utils x11-utils xdotool matchbox-window-manager \
      novnc websockify python3 \
      chromium ca-certificates fonts-dejavu-core fonts-noto-cjk fonts-noto-color-emoji fonts-unifont \
      curl procps supervisor \
    ; \
    fc-cache -f; \
    sed -i 's/^# \(en_US.UTF-8\)/\1/' /etc/locale.gen; \
    locale-gen en_US.UTF-8; \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -s /bin/bash app && \
    mkdir -p /home/app/.vnc /opt/novnc && \
    chown -R app:app /home/app /opt/novnc

## No window manager required; Chromium runs directly on Xvnc

# noVNC helper symlink for convenience
RUN ln -s /usr/share/novnc/utils/novnc_proxy /usr/local/bin/novnc_proxy

# Startup script: launches TigerVNC (:1), noVNC (6901, HTTP), and Chromium
ADD start-vnc.sh /usr/local/bin/start-vnc.sh
RUN chmod +x /usr/local/bin/start-vnc.sh && chown app:app /usr/local/bin/start-vnc.sh

# Helper to keep Chromium window fitted to desktop
ADD x-auto-fit.sh /usr/local/bin/x-auto-fit.sh
RUN chmod +x /usr/local/bin/x-auto-fit.sh && chown app:app /usr/local/bin/x-auto-fit.sh

# Provide a stub fbsetbg to stop wallpaper popups
ADD fbsetbg /usr/local/bin/fbsetbg
RUN chmod +x /usr/local/bin/fbsetbg

# Default landing page: redirect "/" to "/vnc.html?autoconnect=1"
ADD novnc-index.html /usr/share/novnc/index.html

USER app
WORKDIR /home/app

# Do not EXPOSE 5901 by design; only 6901 is relevant to noVNC
EXPOSE 6901

ENTRYPOINT ["/usr/local/bin/start-vnc.sh"]
