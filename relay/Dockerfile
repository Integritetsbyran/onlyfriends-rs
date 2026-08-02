##################
### BASE STAGE ###
##################
FROM rust:1.97.1 AS base

ARG TARGETARCH # https://docs.docker.com/build/building/multi-platform/
RUN if [ "$TARGETARCH" = "" ];      then echo  "x86_64-unknown-linux-musl" | tee /rust-target; fi
RUN if [ "$TARGETARCH" = "amd64" ]; then echo  "x86_64-unknown-linux-musl" | tee /rust-target; fi
RUN if [ "$TARGETARCH" = "arm64" ]; then echo "aarch64-unknown-linux-musl" | tee /rust-target; fi

# Install build dependencies
RUN rustup target add $(cat /rust-target)

# required by "ring"
RUN apt-get update && apt-get install -y musl-tools

RUN mkdir /out
WORKDIR /app

###################
### BUILD STAGE ###
###################
FROM base AS build

COPY . .

RUN cargo build --release --locked --target $(cat /rust-target) -p onlyfriends-relay
RUN mv /app/target/$(cat /rust-target)/release/onlyfriends-relay /out/onlyfriends-relay
RUN strip /out/onlyfriends-relay

########################
### PRODUCTION STAGE ###
########################
FROM scratch

ENV OF_RELAY_BIND="0.0.0.0:3000"
EXPOSE 3000/tcp

# 7 days
ENV OF_MAILBOX_MAX_AGE="604800"

ENV RUST_LOG="info"

WORKDIR /

# Copy application binary
COPY --from=build /out/onlyfriends-relay /

CMD ["/onlyfriends-relay"]
