FROM rust:slim-buster

RUN apt-get update \
    && apt-get install -y --no-install-recommends wget \
    && rm -rf /var/lib/apt/lists/*

# toml-cli
RUN mkdir /build \
    && cd /build/ \
    && wget https://github.com/gnprice/toml-cli/archive/refs/tags/v0.2.0.tar.gz \
    && tar xfz v0.2.0.tar.gz \
    && cd toml-cli-0.2.0 \
    && cargo build --release \
    && mv target/release/toml /usr/local/bin \
    && cd \
    && rm -rf /build \
    && rm -rf ${CARGO_HOME}/registry

COPY sh/ /ics-dm-sh/
ENV PATH="/ics-dm-sh:${PATH}"
