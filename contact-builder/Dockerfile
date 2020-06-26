FROM rust

LABEL description="Container for builds"

RUN rustup default stable
RUN rustup target add wasm32-unknown-unknown

RUN apt-get update && apt-get install -y git less

