FROM rust:1.93.0-bookworm AS base

WORKDIR /app

RUN cargo install cargo-chef --locked --version 0.1.72

FROM base AS planner

COPY . .

RUN cargo chef prepare --recipe-path recipe.json


FROM base AS builder

COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --package hikari-server --package hikari-worker --package hikari-cli --recipe-path recipe.json

COPY . ./

RUN cargo build --release --package hikari-server --package hikari-worker --package hikari-cli

RUN find . -name hikari-server && find . -name hikari-worker && find . -name hikari-cli

FROM debian:bookworm-20260202-slim AS runtime

RUN apt-get -y update && apt-get -y upgrade && apt-get -y install sqlite3 libpq-dev ca-certificates tini

ENV HIKARI_USER=hikari
ENV HIKARI_UID=890
ENV HIKARI_GID=891
ENV PATH="${PATH}:/app"

RUN groupadd --system --gid $HIKARI_GID $HIKARI_USER && useradd --system --no-create-home --gid $HIKARI_GID --uid $HIKARI_UID $HIKARI_USER
COPY --from=builder /app/target/release/hikari-server /app/target/release/hikari-cli /app/target/release/hikari-worker /app/

RUN chown $HIKARI_UID:$HIKARI_GID /app/hikari-server /app/hikari-cli /app/hikari-worker

USER $HIKARI_USER

ENTRYPOINT ["/usr/bin/tini", "--"]
