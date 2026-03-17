#!/bin/bash -eu

# Disable aws-lc-sys jitter entropy which conflicts with fuzzer CFLAGS
export AWS_LC_SYS_NO_JITTER_ENTROPY=1

# Note: This project creates Rust fuzz targets exclusively
cd $SRC
cargo fuzz build -O

FUZZ_TARGET_OUTPUT_DIR=fuzz/target/x86_64-unknown-linux-gnu/release/
for f in fuzz/fuzz_targets/*.rs
do
    FUZZ_TARGET_NAME=$(basename ${f%.*})
    cp $FUZZ_TARGET_OUTPUT_DIR/$FUZZ_TARGET_NAME $OUT/
done

