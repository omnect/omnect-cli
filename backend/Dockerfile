FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
        bmap-tools \
        e2tools \
        fdisk \
        mtools \
        uuid-runtime \
        && rm -rf /var/lib/apt/lists/*

COPY sh/ /omnect-sh/
ENV PATH="/omnect-sh:${PATH}"

RUN mkdir /install
COPY omnect-cli /install/
