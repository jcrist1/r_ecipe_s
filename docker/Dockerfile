FROM --platform=linux/amd64 debian:bullseye-slim

RUN mkdir -p r_ecipe_s_frontend/.perseus/
COPY dist r_ecipe_s_frontend/.perseus/dist
COPY index.html r_ecipe_s_frontend/
COPY static r_ecipe_s_frontend/static

RUN mkdir -p backend/config
COPY main backend/main
COPY config.toml backend/config/config.toml

COPY entrypoint.sh .
CMD ["bash", "./entrypoint.sh"]
