FROM nginx:1.27-alpine

COPY docs/ /usr/share/nginx/html/
COPY deploy/nginx.conf.template /etc/nginx/templates/default.conf.template

ENV PORT=8080
ENV DOWNLOAD_BUCKET=resource-monitor-downloads
EXPOSE 8080
