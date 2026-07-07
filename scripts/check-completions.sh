#!/usr/bin/env bash
set -euo pipefail

COMPLETIONS_DIR="protonup-rs/completions"
EXPECTED_FILES=(
    "protonup-rs.bash"
    "protonup-rs.fish"
    "_protonup-rs"
)

if [[ ! -d "$COMPLETIONS_DIR" ]]; then
    echo "ERROR: $COMPLETIONS_DIR directory is missing."
    echo "Run 'cargo build -p protonup-rs' to generate completion files."
    exit 1
fi

missing=0
for file in "${EXPECTED_FILES[@]}"; do
    if [[ ! -f "$COMPLETIONS_DIR/$file" ]]; then
        echo "ERROR: $COMPLETIONS_DIR/$file is missing."
        missing=1
    fi
done

if [[ $missing -eq 1 ]]; then
    echo "Run 'cargo build -p protonup-rs' to regenerate completion files."
    exit 1
fi

echo "Completion files OK."
