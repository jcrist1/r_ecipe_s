FROM --platform=linux/amd64 debian:bullseye-slim

RUN mkdir -p frontend/
COPY dist frontend/dist
COPY static frontend/static

RUN mkdir -p backend/config
COPY r_ecipe_s_server backend/r_ecipe_s_server
COPY config.toml backend/config/config.toml

COPY entrypoint.sh .
CMD ["bash", "./entrypoint.sh"]
