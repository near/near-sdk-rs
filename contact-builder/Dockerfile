FROM rust:1.44.1

LABEL description="Container for builds"

RUN rustup default stable
RUN rustup target add wasm32-unknown-unknown

RUN apt-get update && apt-get install -y git less vim

