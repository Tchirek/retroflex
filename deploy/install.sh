#!/bin/sh
# 一息に　駅の辻をば　立てる文
set -eu
[ $# -eq 3 ] || { echo "usage: sh install.sh <retroflex-binary> <tunnel-credentials.json> <hostname>" >&2; exit 2; }
bin=$1 creds=$2 host=$3
tunnel=$(basename "$creds" .json)
port=${RETROFLEX_PORT:-7746}

install -d -m 755 /opt/retroflex
install -d -m 755 /etc/retroflex
install -m 755 "$bin" /opt/retroflex/retroflex
install -m 600 "$creds" /etc/retroflex/tunnel.json

if [ ! -x /opt/retroflex/cloudflared ]; then
  arch=$(uname -m); case $arch in x86_64) arch=amd64 ;; aarch64) arch=arm64 ;; esac
  curl -fsSL -o /opt/retroflex/cloudflared "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-$arch"
  chmod 755 /opt/retroflex/cloudflared
fi

if [ ! -f /etc/retroflex/hub.env ]; then
  token=$(head -c 24 /dev/urandom | base64 | tr -d '/+=' | cut -c1-32)
  printf 'RETROFLEX_LISTEN=127.0.0.1:%s\nRETROFLEX_HUB_TOKEN=%s\n' "$port" "$token" > /etc/retroflex/hub.env
  chmod 600 /etc/retroflex/hub.env
fi

cat > /etc/retroflex/cloudflared.yml <<YML
tunnel: $tunnel
ingress:
  - hostname: $host
    service: http://127.0.0.1:$port
  - service: http_status:404
YML
chmod 644 /etc/retroflex/cloudflared.yml

here=$(dirname "$0")
install -m 644 "$here/retroflex-hub.service" "$here/retroflex-tunnel.service" /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now retroflex-hub retroflex-tunnel
systemctl restart retroflex-hub retroflex-tunnel
echo "hub token: $(sed -n 's/^RETROFLEX_HUB_TOKEN=//p' /etc/retroflex/hub.env)"
