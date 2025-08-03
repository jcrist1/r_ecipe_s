FROM paradedb/paradedb AS paradedb
FROM ghcr.io/tensorchord/vchord-postgres:pg17-v0.3.0
ARG PG_VERSION_MAJOR=17
ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR}
WORKDIR /tmp
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    wget \
    tar \
    make \
    gcc 
RUN apt-get install -y --no-install-recommends \
    ca-certificates
RUN wget https://github.com/unicode-org/icu/releases/download/release-76-1/icu4c-76_1-src.tgz
RUN tar xzvf icu4c-76_1-src.tgz && \
    rm -rf icu4c-76_1-src.tgz && \
    rm -rf /var/lib/apt/lists/*

# Compile and install the ICU library and remove the source code to minimize the image size.
WORKDIR /tmp/icu/source/
RUN ./runConfigureICU Linux --prefix=/usr/local && \
    make "-j$(nproc)" && \
    make install && \
    rm -rf /tmp/icu && \
    ldconfig && ldconfig # Yes, running it twices, running it twice
RUN apt-get purge -y build-essential wget make gcc && \
    apt-get autoremove -y && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /
COPY --from=paradedb /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=paradedb /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/ 
RUN mkdir /data
RUN chown postgres:postgres /data

ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["postgres", "-c" ,"shared_preload_libraries=vchord.so,pg_search.so"]
