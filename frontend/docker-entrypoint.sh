#!/bin/sh
set -e

# Extract the first IPv4 nameserver from resolv.conf for nginx resolver directive
# (skip IPv6 addresses which need bracket notation that complicates the config)
DNS_RESOLVER=$(awk '/^nameserver/{ if ($2 !~ /:/) {print $2; exit} }' /etc/resolv.conf)

# Fallback: if no IPv4 nameserver found, try IPv6 with brackets
if [ -z "$DNS_RESOLVER" ]; then
    RAW=$(awk '/^nameserver/{print $2; exit}' /etc/resolv.conf)
    DNS_RESOLVER="[$RAW]"
fi

export DNS_RESOLVER

echo "Using DNS resolver: $DNS_RESOLVER"
echo "Backend URL: $BACKEND_URL"

# Substitute only our variables, leaving nginx variables ($host, $uri, etc.) intact
envsubst '$BACKEND_URL $PORT $DNS_RESOLVER' < /etc/nginx/nginx.conf.template > /etc/nginx/conf.d/default.conf

echo "Generated nginx config:"
cat /etc/nginx/conf.d/default.conf

exec nginx -g 'daemon off;'
