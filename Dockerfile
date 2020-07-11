FROM rust:1.44-slim as build
WORKDIR /app
RUN apt-get update \
    && apt-get install -y musl-tools \
    && rustup target add x86_64-unknown-linux-musl
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine as rt
WORKDIR /app
COPY --from=build /app/target/x86_64-unknown-linux-musl/release .
ENTRYPOINT [ "/app/multiplex" ]
