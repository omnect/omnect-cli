FROM rust:slim-buster

COPY sh/ /ics-dm-sh/
ENV PATH="/ics-dm-sh:${PATH}"

RUN mkdir /install
COPY ics-dm-cli /install/
