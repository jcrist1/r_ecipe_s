FROM --platform=linux/amd64 debian:bullseye-20230919-slim

RUN mkdir -p frontend_ls/
COPY dist frontend_ls/dist
COPY static frontend_ls/static

RUN mkdir -p backend/config
COPY r_ecipe_s_server backend/r_ecipe_s_server
COPY config.toml backend/config/config.toml

COPY entrypoint.sh .
CMD ["bash", "./entrypoint.sh"]
