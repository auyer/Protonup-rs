FROM gcr.io/oss-fuzz-base/base-builder-rust@sha256:37f879129ff67bea6ead5c06ae6e910d6ec27b1c9f85314e212ce005c3e57beb

ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER clang
ENV RUSTUP_TOOLCHAIN nightly-2025-04-05

RUN rm -rf /rust/rustup/toolchains/nightly-2024-07-12-x86_64-unknown-linux-gnu && \
		rustup default nightly && rustup toolchain install nightly-2025-04-05 && \
		rustup component add rust-src && \
		cargo install cargo-fuzz && \
		# remoevs default toolchain
		rm -rf /rust/rustup/toolchains/nightly-x86_64-unknown-linux-gnu && mkdir -p /rust/rustup/toolchains/nightly-x86_64-unknown-linux-gnu && \
		# copy to where the Fuzz builder expects to find the toolchain
		cp -r /rust/rustup/toolchains/nightly-2025-04-05-x86_64-unknown-linux-gnu/* /rust/rustup/toolchains/nightly-x86_64-unknown-linux-gnu/ 

# Copy your project's source code.
COPY . $SRC/
# Working directory for build.sh.
WORKDIR $SRC/
# Copy build.sh into $SRC dir.
COPY ./.clusterfuzzlite/build.sh $SRC/

