FROM gcr.io/oss-fuzz-base/base-builder-rust@sha256:73c1d5648db54100639339d411a5d192cbc8bf413ee91e843a07cf6f0e319dc7 

ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER clang
ENV RUSTUP_TOOLCHAIN nightly-2025-12-28

RUN rm -rf /rust/rustup/toolchains/nightly-2024-07-12-x86_64-unknown-linux-gnu && \
	rustup default nightly && rustup toolchain install nightly-2025-12-28 && \
	rustup component add rust-src && \
	cargo install cargo-fuzz && \
	# remoevs default toolchain
	rm -rf /rust/rustup/toolchains/nightly-x86_64-unknown-linux-gnu && mkdir -p /rust/rustup/toolchains/nightly-x86_64-unknown-linux-gnu && \
	# copy to where the Fuzz builder expects to find the toolchain
	cp -r /rust/rustup/toolchains/nightly-2025-12-28-x86_64-unknown-linux-gnu/* /rust/rustup/toolchains/nightly-x86_64-unknown-linux-gnu/ 

# Copy your project's source code.
COPY . $SRC/
# Working directory for build.sh.
WORKDIR $SRC/
# Copy build.sh into $SRC dir.
COPY ./.clusterfuzzlite/build.sh $SRC/

