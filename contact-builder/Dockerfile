FROM rust:1.47.0

LABEL description="Container for builds"

RUN rustup default stable-2020-10-08
RUN rustup target add wasm32-unknown-unknown

RUN apt-get update && apt-get install -y git less vim

