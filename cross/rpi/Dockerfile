FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main

RUN dpkg --add-architecture arm64  && \ 
    apt-get update && \
    apt-get install --assume-yes libx11-dev:arm64 libasound2-dev libasound2-dev:arm64 libudev-dev libudev-dev:arm64 gcc-aarch64-linux-gnu g++-aarch64-linux-gnu

ENV WGPU_OPTIONS_PRIO="webgl2"
