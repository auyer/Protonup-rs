FROM gcr.io/oss-fuzz-base/base-builder-rust

# Copy your project's source code.
COPY . $SRC/
# Working directory for build.sh.
WORKDIR $SRC/
# Copy build.sh into $SRC dir.
COPY ./.clusterfuzzlite/build.sh $SRC/

