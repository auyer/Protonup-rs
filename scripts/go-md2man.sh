#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

INPUT="$REPO_ROOT/protonup-rs/man/protonup-rs.1.md"
OUTPUT="$REPO_ROOT/protonup-rs/man/protonup-rs.1"

if [ ! -f "$INPUT" ]; then
    echo "Error: $INPUT not found" >&2
    exit 1
fi

if ! command -v go-md2man &> /dev/null; then
    echo "Error: go-md2man is not installed" >&2
    exit 1
fi

go-md2man -in="$INPUT" -out="$OUTPUT"

if ! git -C "$REPO_ROOT" ls-files --error-unmatch -- "protonup-rs/man/protonup-rs.1" >/dev/null 2>&1; then
    echo "" >&2
    echo "Error: protonup-rs/man/protonup-rs.1 is not staged for commit." >&2
    echo "Stage it with:  git add protonup-rs/man/protonup-rs.1" >&2
    exit 1
fi

if ! git -C "$REPO_ROOT" diff --quiet --exit-code -- "protonup-rs/man/protonup-rs.1"; then
    echo "" >&2
    echo "Error: protonup-rs/man/protonup-rs.1 is out of date with protonup-rs/man/protonup-rs.1.md." >&2
    echo "Stage the updated man page:  git add protonup-rs/man/protonup-rs.1" >&2
    exit 1
fi
