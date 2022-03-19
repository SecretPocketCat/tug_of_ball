FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main

RUN apt-get update && \
    apt-get install --assume-yes libx11-dev libasound2-dev libudev-dev python-dev gcc-arm-linux-gnueabihf g++-arm-linux-gnueabih wget 

RUN cd $HOME && \
    wget https://developer.arm.com/-/media/Files/downloads/gnu-a/10.3-2021.07/binrel/gcc-arm-10.3-2021.07-x86_64-aarch64-none-linux-gnu.tar.xz && \
    tar -xf gcc-arm-10.3-2021.07-x86_64-aarch64-none-linux-gnu.tar.xz

ENV PATH $HOME/gcc-arm-10.3-2021.07-x86_64-aarch64-none-linux-gnu.tar.xz/bin:$PATH
