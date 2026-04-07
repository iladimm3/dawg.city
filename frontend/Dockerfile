# Build stage
FROM node:20-alpine AS builder
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --legacy-peer-deps
COPY . .
RUN npm run build

# Production stage
FROM nginx:alpine
RUN rm /etc/nginx/conf.d/default.conf
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf.template /etc/nginx/nginx.conf.template

# Extract the container's DNS resolver at runtime (works on Railway, Docker, etc.)
# Then substitute all template variables, leaving Nginx variables ($host, $uri, etc.) intact
CMD ["sh", "-c", "export DNS_RESOLVER=$(grep nameserver /etc/resolv.conf | head -1 | awk '{print $2}') && envsubst '$BACKEND_URL $PORT $DNS_RESOLVER' < /etc/nginx/nginx.conf.template > /etc/nginx/conf.d/default.conf && exec nginx -g 'daemon off;'"]
