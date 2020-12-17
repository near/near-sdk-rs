# our local base image
# FROM ubuntu
FROM rust

LABEL description="Container for builds"

# RUN rustup update nightly
RUN rustup toolchain uninstall nightly
# Must match nearcore/rust-toolchain.
RUN rustup default nightly-2020-05-15


RUN apt-get update
RUN apt-get upgrade -y
# install build dependencies for QEMU
RUN  apt-get install -y  g++ rsync zip openssh-server \
     make apt-utils git sudo pkg-config libglib2.0-dev curl clang gdb \
     valgrind jq

