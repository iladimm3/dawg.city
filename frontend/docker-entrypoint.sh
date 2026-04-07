#!/bin/sh
set -e

# Extract the first nameserver from resolv.conf for nginx resolver directive
DNS_RESOLVER=$(awk '/^nameserver/{print $2; exit}' /etc/resolv.conf)
export DNS_RESOLVER

echo "Using DNS resolver: $DNS_RESOLVER"
echo "Backend URL: $BACKEND_URL"

# Substitute only our variables, leaving nginx variables ($host, $uri, etc.) intact
envsubst '$BACKEND_URL $PORT $DNS_RESOLVER' < /etc/nginx/nginx.conf.template > /etc/nginx/conf.d/default.conf

echo "Generated nginx config:"
cat /etc/nginx/conf.d/default.conf

exec nginx -g 'daemon off;'
