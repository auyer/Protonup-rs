#!/bin/bash

# default repo to check is GEProton
REPO="${1:-"GloriousEggroll/proton-ge-custom"}"

TAG_NAME=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | jq -r '.tag_name')

if [[ "$TAG_NAME" == "null" || -z "$TAG_NAME" ]]; then
    echo "Error: Could not fetch the latest release for $REPO." >&2
    exit 1
fi

# Strip all leading non-digit characters from the tag string
# - 'GE-Proton11-5' becomes '11-5'
VERSION=$(echo "$TAG_NAME" | sed -E 's/^[^0-9]+//')

echo "$VERSION"
