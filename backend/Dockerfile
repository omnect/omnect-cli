FROM debian AS builder
ENV DEBIAN_FRONTEND noninteractive
RUN apt-get update && apt-get install -y\
        build-essential\
        libext2fs-dev\
        pkg-config \
        wget
RUN wget https://github.com/e2tools/e2tools/releases/download/v0.1.0/e2tools-0.1.0.tar.gz && tar xfz e2tools-0.1.0.tar.gz
WORKDIR e2tools-0.1.0
RUN ./configure && make -j$(getconf _NPROCESSORS_ONLN) && make install

###############################################################################
FROM debian:bullseye-slim

COPY --from=builder /usr/local/bin/* /usr/local/bin/

RUN apt-get update && apt-get install -y \
        fdisk \
        uuid-runtime \
    && rm -rf /var/lib/apt/lists/*

COPY sh/ /ics-dm-sh/
ENV PATH="/ics-dm-sh:${PATH}"

RUN mkdir /install
COPY ics-dm-cli /install/
