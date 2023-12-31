#!/bin/bash

## run me :
# curl -sSL https://raw.githubusercontent.com/zhouzhipeng/play/main/scripts/install_service.sh | sudo bash

systemctl stop play

set -eux

cd /root

rm -rf play

curl -o play https://github.com/zhouzhipeng/play/releases/download/latest/play_linux
chmod +x play

# register service
cat > /etc/systemd/system/play.service << EOF
[Unit]
Description=Play Service
After=network.target

[Service]
Type=simple
Restart=always
RestartSec=3
ExecStart=/root/play

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload

systemctl enable play.service

systemctl start play

systemctl status play