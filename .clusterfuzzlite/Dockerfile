FROM gcr.io/oss-fuzz-base/base-builder
#FROM gcr.io/oss-fuzz-base/base-builder-rust

RUN apt-get update && apt-get install -y make autoconf automake libtool curl cmake python llvm-dev libclang-dev clang libc++-dev libc++abi-dev lld

ENV PATH="/root/.cargo/bin:${PATH}"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y --default-toolchain nightly && \
		rustup component add rust-src && \
		cargo install cargo-fuzz

# copy toolchain to where the action fuzzer expects to find it
RUN mkdir -p /rust/rustup/toolchains/ && cp -r ~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu /rust/rustup/toolchains/


# RUN rustup default nightly && rustup update
# Copy your project's source code.
COPY . $SRC/
# Working directory for build.sh.
WORKDIR $SRC/
# Copy build.sh into $SRC dir.
COPY ./.clusterfuzzlite/build.sh $SRC/

