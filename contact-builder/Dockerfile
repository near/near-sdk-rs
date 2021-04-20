FROM rust:1.51.0

LABEL description="Container for builds"

RUN rustup default stable-2021-03-25
RUN rustup target add wasm32-unknown-unknown

RUN apt-get update && apt-get install -y git less vim

