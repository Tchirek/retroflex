#!/bin/sh
# 秘めし鍵　画面に出さず　書き留める
set -eu
umask 077
printf 'R2 account ID: '
read -r account
printf 'Bucket name [retroflex]: '
read -r bucket
bucket=${bucket:-retroflex}
printf 'Access Key ID: '
read -r key
printf 'Secret Access Key (hidden): '
stty -echo
read -r secret
stty echo
printf '\n'
[ -n "$account" ] && [ -n "$key" ] && [ -n "$secret" ] || { echo "missing value" >&2; exit 1; }
install -d -m 755 /etc/retroflex
cat > /etc/retroflex/r2.env <<ENV
RETROFLEX_S3_ENDPOINT=https://$account.r2.cloudflarestorage.com
RETROFLEX_S3_KEY=$key
RETROFLEX_S3_SECRET=$secret
RETROFLEX_S3_BUCKET=$bucket
ENV
chmod 600 /etc/retroflex/r2.env
echo "saved /etc/retroflex/r2.env"
