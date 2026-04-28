#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -h, --help       Show this help message"
    echo "  -v, --version    Set specific version (e.g., 0.13.0)"
    echo "  -p, --patch      Bump patch version (0.12.0 -> 0.12.1)"
    echo "  -m, --minor      Bump minor version (0.12.0 -> 0.13.0)"
    echo ""
    echo "If no option is provided, interactive mode is enabled."
    exit 0
}

get_current_version() {
    grep -m1 '^version' libprotonup/Cargo.toml | sed 's/version = "$.*$"/\1/'
}

bump_patch() {
    local version=$1
    local major=$(echo "$version" | cut -d. -f1)
    local minor=$(echo "$version" | cut -d. -f2)
    local patch=$(echo "$version" | cut -d. -f3)
    echo "${major}.${minor}.$((patch + 1))"
}

bump_minor() {
    local version=$1
    local major=$(echo "$version" | cut -d. -f1)
    local minor=$(echo "$version" | cut -d. -f2)
    echo "${major}.$((minor + 1)).0"
}

OLD_VERSION=$(grep -m1 '^version' libprotonup/Cargo.toml | sed -E 's/version = "([^"]+)"/\1/')
echo -e "${GREEN}Current version: $OLD_VERSION${NC}"

if [[ $# -eq 0 ]]; then
    echo ""
    echo "Select bump type:"
    echo "  1) Patch (bugfix): $OLD_VERSION -> $(bump_patch $OLD_VERSION)"
    echo "  2) Minor (feature): $OLD_VERSION -> $(bump_minor $OLD_VERSION)"
    echo "  3) Custom version"
    echo ""
    read -p "Choice [1-3]: " choice

    case $choice in
        1) NEW_VERSION=$(bump_patch $OLD_VERSION);;
        2) NEW_VERSION=$(bump_minor $OLD_VERSION);;
        3) read -p "Enter new version (e.g., 0.13.0): " NEW_VERSION;;
        *) echo -e "${RED}Invalid choice${NC}"; exit 1;;
    esac
else
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help) usage;;
            -v|--version) NEW_VERSION="$2"; shift 2;;
            -p|--patch) NEW_VERSION=$(bump_patch $OLD_VERSION); shift;;
            -m|--minor) NEW_VERSION=$(bump_minor $OLD_VERSION); shift;;
            *) echo -e "${RED}Unknown option: $1${NC}"; usage;;
        esac
    done
fi

if [[ -z "$NEW_VERSION" ]]; then
    echo -e "${RED}No version specified${NC}"
    exit 1
fi

echo -e "${YELLOW}Bumping version: $OLD_VERSION -> $NEW_VERSION${NC}"

echo -e "${GREEN}Updating libprotonup/Cargo.toml${NC}"
sed -i "s/^version = \"$OLD_VERSION\"/version = \"$NEW_VERSION\"/" libprotonup/Cargo.toml

echo -e "${GREEN}Updating protonup-rs/Cargo.toml${NC}"
sed -i "s/^version = \"$OLD_VERSION\"/version = \"$NEW_VERSION\"/" protonup-rs/Cargo.toml
sed -i "s|libprotonup = { path = \"\.\./libprotonup\", version = \"$OLD_VERSION\" }|libprotonup = { path = \"../libprotonup\", version = \"$NEW_VERSION\" }|" protonup-rs/Cargo.toml

echo -e "${GREEN}Updating Cargo.lock files${NC}"
cargo update -p libprotonup --precise "$NEW_VERSION" 2>/dev/null || true
cargo update -p protonup-rs --precise "$NEW_VERSION" 2>/dev/null || true

echo -e "${GREEN}Updating fuzz crate dependencies${NC}"
cd fuzz && cargo update -p libprotonup --precise "$NEW_VERSION" 2>/dev/null || true
cd ..

echo -e "${GREEN}Version bumped to $NEW_VERSION${NC}"
echo -e "${YELLOW}Please review changes with 'git diff' before committing${NC}"
