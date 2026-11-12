#!/usr/bin/env bash
# Run the GUI end-to-end test inside a container built from
# .devcontainer/Containerfile.
#
# Usage:
#   scripts/gui_e2e_container.sh [--rebuild]
#
# Why: the e2e needs a specific X11 runtime stack plus a headless X server, and
# on some desktops synthetic key events do not reach the app. A container gives
# a known-good Debian environment with its own Xvfb, independent of the host.
#
# The repository is mounted read-only; build artifacts and the cargo cache live
# in named volumes, so nothing on the host is modified and rebuilds are cheap.
#
# Requires podman (preferred) or docker. Override with CONTAINER_ENGINE=docker.
#
# Environment variables forwarded into the container:
#   WAIT_SECONDS, KEY_DELAY, TITLE, E2E_KEEP, ICED_BACKEND, DISPLAY_NUM
# See scripts/gui_e2e_interactive.sh for what they do.
set -euo pipefail

REPO_ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
CONTAINERFILE="$REPO_ROOT/.devcontainer/Containerfile"
IMAGE="${IMAGE:-protonup-rs-gui-e2e}"
# Persisting these keeps the second run fast.
TARGET_VOLUME="${TARGET_VOLUME:-protonup-rs-gui-e2e-target}"
REGISTRY_VOLUME="${REGISTRY_VOLUME:-protonup-rs-gui-e2e-cargo}"

usage() {
    cat <<'EOF'
Usage: scripts/gui_e2e_container.sh [--rebuild]

Builds (once) an image from .devcontainer/Containerfile and runs
scripts/gui_e2e_interactive.sh inside it, against a throwaway HOME.

Options:
  --rebuild   rebuild the image before running
  -h, --help  show this help

Environment:
  CONTAINER_ENGINE  force podman or docker (default: first one found)
  IMAGE             image tag (default: protonup-rs-gui-e2e)
  TARGET_VOLUME     volume for ./target      (default: protonup-rs-gui-e2e-target)
  REGISTRY_VOLUME   volume for the cargo cache (default: protonup-rs-gui-e2e-cargo)
  WAIT_SECONDS, KEY_DELAY, TITLE, E2E_KEEP, ICED_BACKEND, DISPLAY_NUM
                    forwarded to the test script
EOF
}

REBUILD=0
while [ $# -gt 0 ]; do
    case "$1" in
        --rebuild) REBUILD=1 ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            echo "ERROR: unknown argument '$1'" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

# ---- Container engine --------------------------------------------------------
if [ -n "${CONTAINER_ENGINE:-}" ]; then
    ENGINE="$CONTAINER_ENGINE"
    if ! command -v "$ENGINE" > /dev/null 2>&1; then
        echo "ERROR: CONTAINER_ENGINE=$ENGINE was not found in PATH." >&2
        exit 1
    fi
elif command -v podman > /dev/null 2>&1; then
    ENGINE=podman
elif command -v docker > /dev/null 2>&1; then
    ENGINE=docker
else
    echo "ERROR: neither podman nor docker is installed." >&2
    echo "Install one of them, or set CONTAINER_ENGINE to a compatible CLI." >&2
    exit 1
fi

if [ ! -f "$CONTAINERFILE" ]; then
    echo "ERROR: $CONTAINERFILE not found." >&2
    exit 1
fi

# SELinux hosts need a label relabel for the bind mount; podman supports :Z.
ENGINE_NAME=$(basename -- "$ENGINE")
if [ "$ENGINE_NAME" = podman ]; then
    MOUNT_OPTS="ro,Z"
else
    MOUNT_OPTS="ro"
fi

echo "Engine: $ENGINE"
echo "Image:  $IMAGE"

# ---- Build -------------------------------------------------------------------
if [ "$REBUILD" -eq 1 ] || ! "$ENGINE" image inspect "$IMAGE" > /dev/null 2>&1; then
    echo "Building $IMAGE from $CONTAINERFILE ..."
    "$ENGINE" build -t "$IMAGE" -f "$CONTAINERFILE" "$REPO_ROOT/.devcontainer"
else
    echo "Reusing existing $IMAGE (pass --rebuild to rebuild)."
fi

# ---- Run ---------------------------------------------------------------------
# Forward only the knobs the test understands; DISPLAY is deliberately NOT
# forwarded so the container always starts its own Xvfb.
RUN_ENV=()
for var in WAIT_SECONDS KEY_DELAY TITLE E2E_KEEP ICED_BACKEND DISPLAY_NUM; do
    value="${!var:-}"
    if [ -n "$value" ]; then
        RUN_ENV+=(-e "$var=$value")
    fi
done

# The repository is read-only; the build writes to a volume mounted over
# ./target, so the host tree stays untouched. `--locked` makes a stale
# Cargo.lock fail loudly rather than trying to rewrite it.
echo "Running the GUI e2e; the first run downloads a GEProton release (a few hundred MB) ..."
"$ENGINE" run --rm \
    -v "$REPO_ROOT:/work:$MOUNT_OPTS" \
    -v "$TARGET_VOLUME:/work/target" \
    -v "$REGISTRY_VOLUME:/opt/rust/registry" \
    ${RUN_ENV[@]+"${RUN_ENV[@]}"} \
    -w /work \
    "$IMAGE" \
    bash -c 'set -euo pipefail; cargo build --release -p protonup-rsui --locked && exec bash scripts/gui_e2e_interactive.sh ./target/release/protonup-rsui'
